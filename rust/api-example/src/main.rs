use keycloak_api::{auth::DirectGrantAuth, prelude::*};

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    let auth = DirectGrantAuth::new(
        "admin-cli".into(),
        None,
        &std::env::var("KEYCLOAK_USERNAME")?,
        &std::env::var("KEYCLOAK_PASSWORD")?,
    );
    let kc = Keycloak::new(
        &std::env::var("KEYCLOAK_BASE_URL")?,
        &std::env::var("KEYCLOAK_REALM")?,
        auth,
    )
    .await?;
    let info = kc.server_info().await?;
    println!("server info: {info:#?}");
    let info = kc.realm_info().await?;
    println!("realm info: {info:#?}");
    let Err(e) = kc.with_client(test).await else {
        color_eyre::eyre::bail!("expected invalid realm call to fail!");
    };
    println!(
        "report for expected error:\n{:?}",
        color_eyre::Report::new(e)
    );
    kc.with_client_boxed_future(|client| Box::pin(async move { test(client).await }))
        .await
        .expect_err("same thing");

    println!("tests passed");
    Ok(())
}

async fn test(client: &keycloak_api::rest::Client) -> Result<(), keycloak_api::Error> {
    client
        .get_realm("this_realm_is_invalid_and_does_not_exist")
        .await
        .map_err(keycloak_api::error::progenitor)?;
    Ok(())
}
