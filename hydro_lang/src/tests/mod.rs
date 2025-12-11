//! hydro_lang integration tests

#[cfg(all(feature = "deploy", feature = "docker_deploy"))]
#[cfg(test)]
pub mod hydro_deploy;
