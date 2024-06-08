use crate::watcher::watcher;
use futures::{pin_mut, TryStreamExt};
use k8s_openapi::api::core::v1::{Event, Pod};
use kube::{
    api::{Api, PostParams, ResourceExt},
    runtime::{
        reflector::Lookup,
        watcher::{self},
        WatchStreamExt,
    },
    Client,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::all(client.clone());
    let events: Api<Event> = Api::all(client.clone());

    let config = kube::runtime::watcher::Config::default();
    let stream = watcher(pods, config).applied_objects();
    pin_mut!(stream);

    while let Some(pod) = stream.try_next().await? {
        if let Some(status) = &pod.status {
            if let Some(container_statuses) = &status.container_statuses {
                for container_status in container_statuses {
                    if let Some(state) = &container_status.last_state {
                        if let Some(terminated) = &state.terminated {
                            if terminated.reason.as_deref() == Some("OOMKilled") {
                                //let namespace = Lookup::namespace(&pod).unwrap_or_default();
                                let namespace = ResourceExt::namespace(&pod).unwrap_or_default();
                                //let namespace = pod.namespace().unwrap_or_default();
                                let pod_name = pod.name().unwrap_or_default();
                                let message = format!(
                                    "Pod {} in namespace {} was OOMKilled.",
                                    pod_name, namespace
                                );
                                println!("{}", message);
                                create_event(&events, &namespace, &pod_name, &message).await?;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

async fn create_event(
    events: &Api<Event>,
    namespace: &str,
    pod_name: &str,
    message: &str,
) -> anyhow::Result<()> {
    let event = Event {
        metadata: kube::api::ObjectMeta {
            name: Some(format!("{}.oomkill", pod_name)),
            namespace: Some(namespace.to_string()),
            ..Default::default()
        },
        involved_object: k8s_openapi::api::core::v1::ObjectReference {
            kind: Some("Pod".to_string()),
            namespace: Some(namespace.to_string()),
            name: Some(pod_name.to_string()),
            ..Default::default()
        },
        reason: Some("OOMKilled".to_string()),
        message: Some(message.to_string()),
        type_: Some("Warning".to_string()),
        source: Some(k8s_openapi::api::core::v1::EventSource {
            component: Some("oom-listener".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let pp = PostParams::default();
    match events.create(&pp, &event).await {
        Ok(_) => println!("Created event for {} in {}", pod_name, namespace),
        Err(e) => eprintln!("Error creating event: {:?}", e),
    }

    Ok(())
}
