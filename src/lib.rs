use guest::prelude::*;
use k8s_openapi::api::core::v1 as apicore;
use kubewarden_policy_sdk::wapc_guest as guest;
use lazy_static::lazy_static;
use std::collections::hash_map::HashMap;

extern crate kubewarden_policy_sdk as kubewarden;
use kubewarden::{logging, protocol_version_guest, request::ValidationRequest, validate_settings};

mod image;
mod settings;
use image::ImageRef;
use settings::Settings;

use slog::{info, o, warn, Logger};

lazy_static! {
    static ref LOG_DRAIN: Logger = Logger::root(
        logging::KubewardenDrain::new(),
        o!("policy" => "mutate repos policy")
    );
}

#[no_mangle]
pub extern "C" fn wapc_init() {
    register_function("validate", validate);
    register_function("validate_settings", validate_settings::<Settings>);
    register_function("protocol_version", protocol_version_guest);
}

fn validate(payload: &[u8]) -> CallResult {
    let validation_request: ValidationRequest<Settings> = ValidationRequest::new(payload)?;

    info!(LOG_DRAIN, "starting validation");

    // TODO: you can unmarshal any Kubernetes API type you are interested in
    match serde_json::from_value::<apicore::Pod>(validation_request.request.object) {
        Ok(mut pod) => {
            pod = mutate_pod(pod, &validation_request.settings);
            let mutated_object = serde_json::to_value(pod)?;
            kubewarden::mutate_request(mutated_object)
        }
        Err(_) => {
            // TODO: handle as you wish
            // We were forwarded a request we cannot unmarshal or
            // understand, just accept it
            warn!(LOG_DRAIN, "cannot unmarshal resource: this policy does not know how to evaluate this resource; accept it");
            kubewarden::accept_request()
        }
    }
}

fn mutate_pod(mut pod: apicore::Pod, settings: &Settings) -> apicore::Pod {
    let mut pod_spec = pod.spec.unwrap();
    pod_spec.containers = mutate_containers(&pod_spec.containers, settings.repos.clone());
    if let Some(init_containers) = &pod_spec.init_containers {
        pod_spec.init_containers = Some(mutate_containers(init_containers, settings.repos.clone()));
    }
    pod.spec = Some(pod_spec);
    pod
}

fn mutate_containers(
    containers: &[apicore::Container],
    repos: HashMap<String, String>,
) -> Vec<apicore::Container> {
    let ctrs = containers
        .iter()
        .map(|container| {
            let mut ctr = container.clone();
            if let Some(ctr_image) = &ctr.image {
                let image = ImageRef::parse(ctr_image.as_str()).to_string();
                for (src, dest) in repos.clone().into_iter() {
                    if image.starts_with(&src) {
                        ctr.image = Some(image.replace(&src, &dest));
                        break;
                    }
                }
            }
            ctr
        })
        .collect();
    ctrs
}

#[cfg(test)]
mod tests {
    use super::*;

    use kubewarden_policy_sdk::test::Testcase;

    #[test]
    fn mutate_pod_repos() -> Result<(), ()> {
        let request_file = "test_data/pod_creation.json";
        let tc = Testcase {
            name: String::from("Valid name"),
            fixture_file: String::from(request_file),
            expected_validation_result: true,
            settings: Settings {
                repos: HashMap::from([
                    ("quay.io".to_string(), "quay.tencentcloudcr.com".to_string()),
                    ("gcr.io".to_string(), "gcr.tencentcloudcr.com".to_string()),
                    (
                        "docker.io".to_string(),
                        "dockerhub.tencentcloudcr.com".to_string(),
                    ),
                    (
                        "k8s.gcr.io".to_string(),
                        "k8s.tencentcloudcr.com".to_string(),
                    ),
                ]),
            },
        };

        let res = tc.eval(validate).unwrap();
        assert!(
            res.mutated_object.is_some(),
            "Something mutated with test case: {}",
            tc.name,
        );
        info!(LOG_DRAIN, "{}", res.mutated_object.unwrap());

        Ok(())
    }
}
