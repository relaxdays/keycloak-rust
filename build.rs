use std::fs::File;

use openapiv3::OpenAPI;

fn main() {
    let src = std::env::var("OPENAPI_SPEC_PATH").unwrap_or_else(|_| "./openapi.json".into());
    println!("cargo:rerun-if-env-changed=OPENAPI_SPEC_PATH");
    println!("cargo:rerun-if-changed={src}");
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

// all of this is just one giant hack to make sure this works
// the generated openapi spec doesn't match the keycloak api and the rust generator doesn't support all openapi features...
fn fix_spec(spec: &mut OpenAPI) {
    for (path, component) in &mut spec.paths.paths.iter_mut() {
        let item = component.get_item_mut().unwrap();

        fix_operation(&mut item.get, "get", path);
        fix_operation(&mut item.post, "post", path);
        fix_operation(&mut item.put, "put", path);
        fix_operation(&mut item.delete, "delete", path);
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
            .replace("users/{user-id}", "user");

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