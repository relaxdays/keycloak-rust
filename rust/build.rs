use std::fs::File;

use openapiv3::OpenAPI;

fn main() {
    let src = find_spec_path();
    let file = File::open(src).unwrap();
    let mut spec = serde_json::from_reader(file).unwrap();
    fix_spec(&mut spec);

    let mut generator = progenitor::Generator::default();
    let tokens = generator.generate_tokens(&spec).unwrap();
    let ast = syn::parse2(tokens).unwrap();
    let content = prettyplease::unparse(&ast);

    let mut out_file = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).to_path_buf();
    out_file.push("keycloak-api-gen.rs");
    std::fs::write(out_file, content).unwrap();
}

fn find_spec_path() -> std::path::PathBuf {
    println!("cargo:rerun-if-env-changed=OPENAPI_SPEC_PATH");
    let path = if let Ok(src) = std::env::var("OPENAPI_SPEC_PATH") {
        std::path::PathBuf::from(src)
    } else {
        println!("cargo:rerun-if-env-changed=KEYCLOAK_VERSION");
        let mut path = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let kc_version = std::env::var("KEYCLOAK_VERSION").unwrap_or_else(|_| "unstable".into());
        path.extend(["api-spec", kc_version.as_str(), "openapi.json"]);
        path
    };

    if path.exists() {
        // we're constructing the path from nothing but rust strings, so this must be valid utf8
        println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
        return path;
    }
    panic!(
        "openapi spec file not found! set OPENAPI_SPEC_PATH to the openapi.json file from keycloak."
    );
}

// all of this is just one giant hack to make sure this works
// the generated openapi spec doesn't match the keycloak api and the rust generator doesn't support all openapi features...
fn fix_spec(spec: &mut OpenAPI) {
    for (path, component) in &mut spec.paths.paths.iter_mut() {
        let item = component.get_item_mut().unwrap();

        fix_operation(&mut item.get, "get", path);
        fix_operation(&mut item.post, "post", path);
        fix_operation(&mut item.put, "put", path);
        fix_operation(&mut item.delete, "delete", path);

        if path == "/admin/realms/{realm}/clients/{client-uuid}/authz/resource-server/policy/by-type/{policy-type}/{policy-id}" {
            fix_stringly_typed_json_body(&mut item.put);
        }
    }

    let components = spec.components.as_mut().unwrap();
    remove_auth_time(components.schemas.get_mut("AccessToken").unwrap());
    remove_auth_time(components.schemas.get_mut("IDToken").unwrap());
}

fn fix_operation(op: &mut Option<openapiv3::Operation>, r#type: &str, path: &str) {
    let Some(op) = op else {
        return;
    };

    // generate operation ids (optional in openapi spec but required by progenitor to generate function names)
    if op.operation_id.is_none() {
        let path = path.strip_prefix("/admin/").unwrap();
        let path = path
            .replace("clients/{client-uuid}", "client")
            .replace("groups/{group-id}", "group")
            .replace("realms/{realm}", "realm")
            .replace("resource/{resource-id}", "resource-by-id")
            .replace("roles/{role-name}", "role-by-name")
            .replace("roles-by-id/{role-id}", "role-by-id")
            .replace("sessions/{session}", "session")
            .replace("users/{user-id}", "user")
            .replace("permission/by-id/{policy-id}", "permission-by-id")
            .replace("permission/by-type/{policy-type}", "permission-by-type")
            .replace("policy/by-id/{policy-id}", "policy-by-id")
            .replace("policy/by-type/{policy-type}", "policy-by-type");

        op.operation_id = Some(format!("{}_{}", r#type, path.replace('/', "_")));
    }

    if let Some(body) = op.request_body.as_mut().and_then(RefOrExt::get_item_mut) {
        // progenitor only supports 1 media type
        if body.content.len() > 1 {
            let mut keys = body.content.keys().cloned().collect::<Vec<_>>();
            if let Some((i, _)) = keys
                .iter()
                .enumerate()
                .find(|(_, k)| *k == "application/json")
            {
                keys.swap_remove(i);
            } else {
                keys.swap_remove(0);
            }
            for k in keys {
                body.content.swap_remove(&k);
            }
        }
    }

    // progenitor doesn't support operations with multiple success responses
    // -> filter out non-200 responses when multiple 2xx responses exist
    let responses = &mut op.responses.responses;
    if responses.len() > 1 {
        let mut success_keys = responses
            .keys()
            .filter(|s| match s {
                openapiv3::StatusCode::Range(2) => true,
                openapiv3::StatusCode::Code(c) if *c >= 200 && *c < 300 => true,
                _ => false,
            })
            .cloned()
            .collect::<Vec<_>>();
        if success_keys.len() > 1 {
            if let Some((i, _)) = success_keys
                .iter()
                .enumerate()
                .find(|(_, s)| **s == openapiv3::StatusCode::Code(200))
            {
                success_keys.swap_remove(i);
            } else {
                success_keys.pop();
            }
            for k in success_keys {
                responses.swap_remove(&k);
            }
        }
    }

    // remove array parameters (see https://github.com/oxidecomputer/progenitor/issues/268) but only those that are not in path
    op.parameters
        .retain(|p| parameter_is_path_parameter(p) || !parameter_is_array(p));
    // replace array-typed path parameters
    op.parameters.iter_mut().for_each(|p| {
        if !(parameter_is_path_parameter(p) && parameter_is_array(p)) {
            return;
        }
        let openapiv3::ReferenceOr::Item(p) = p else {
            return;
        };
        let d = p.parameter_data_mut();
        let openapiv3::ParameterSchemaOrContent::Schema(openapiv3::ReferenceOr::Item(schema)) =
            &mut d.format
        else {
            return;
        };
        schema.schema_kind =
            openapiv3::SchemaKind::Type(openapiv3::Type::String(openapiv3::StringType {
                ..Default::default()
            }));
    })
}

fn remove_auth_time(schema: &mut openapiv3::ReferenceOr<openapiv3::Schema>) {
    let openapiv3::ReferenceOr::Item(openapiv3::Schema {
        schema_kind:
            openapiv3::SchemaKind::Type(openapiv3::Type::Object(openapiv3::ObjectType {
                properties,
                ..
            })),
        ..
    }) = schema
    else {
        panic!();
    };
    // authTime and auth_time conflict in the generated output
    properties.shift_remove("authTime");
}

fn parameter_is_path_parameter(parameter: &openapiv3::ReferenceOr<openapiv3::Parameter>) -> bool {
    let openapiv3::ReferenceOr::Item(p) = parameter else {
        return false;
    };

    matches!(p, openapiv3::Parameter::Path { .. })
}

fn parameter_is_array(parameter: &openapiv3::ReferenceOr<openapiv3::Parameter>) -> bool {
    let openapiv3::ReferenceOr::Item(p) = parameter else {
        return false;
    };

    let data = p.parameter_data_ref();
    match &data.format {
        openapiv3::ParameterSchemaOrContent::Schema(openapiv3::ReferenceOr::Item(
            openapiv3::Schema {
                schema_kind: openapiv3::SchemaKind::Type(openapiv3::Type::Array(_)),
                ..
            },
        )) => {
            // array type
            true
        }
        _ => false,
    }
}

// some keycloak operations are defined to take a string as body, but actually they expect an object
// one example are different policy types where the concrete object they expect depends on the type given
// in the request path, so the api cannot specify the actual type to use
// string still means that the generated client will wrap a string parameter as another json string which
// keycloak obviously doesn't handle correctly
fn fix_stringly_typed_json_body(operation: &mut Option<openapiv3::Operation>) {
    let op = operation.as_mut().unwrap();
    let body = op.request_body.as_mut().unwrap().get_item_mut().unwrap();
    let json_body = body.content.get_mut("application/json").unwrap();
    let schema = json_body.schema.as_mut().unwrap().get_item_mut().unwrap();
    schema.schema_kind =
        openapiv3::SchemaKind::Type(openapiv3::Type::Object(openapiv3::ObjectType {
            ..Default::default()
        }));
}

trait RefOrExt<T> {
    fn get_item_mut(&mut self) -> Option<&mut T>;
}

impl<T> RefOrExt<T> for openapiv3::ReferenceOr<T> {
    fn get_item_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Item(item) => Some(item),
            _ => None,
        }
    }
}

trait PathExt {
    fn get_path_item_mut(&mut self, path: &str) -> Option<&mut openapiv3::PathItem>;
}

impl PathExt for openapiv3::Paths {
    fn get_path_item_mut(&mut self, path: &str) -> Option<&mut openapiv3::PathItem> {
        self.paths.get_mut(path).and_then(RefOrExt::get_item_mut)
    }
}

trait ParameterExt {
    fn parameter_data_mut(&mut self) -> &mut openapiv3::ParameterData;
}
impl ParameterExt for openapiv3::Parameter {
    fn parameter_data_mut(&mut self) -> &mut openapiv3::ParameterData {
        match self {
            openapiv3::Parameter::Cookie { parameter_data, .. } => parameter_data,
            openapiv3::Parameter::Header { parameter_data, .. } => parameter_data,
            openapiv3::Parameter::Path { parameter_data, .. } => parameter_data,
            openapiv3::Parameter::Query { parameter_data, .. } => parameter_data,
        }
    }
}
