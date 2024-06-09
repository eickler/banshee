use crate::watcher::watcher;
use chrono::Utc;
use futures::{pin_mut, TryStreamExt};
use k8s_openapi::{
    api::core::v1::{Event, EventSource, ObjectReference, Pod},
    apimachinery::pkg::apis::meta::v1::Time,
};
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
        if was_killed(&pod) {
            create_event(&events, &pod).await?;
        }
    }

    Ok(())
}

fn was_killed(pod: &Pod) -> bool {
    if let Some(status) = &pod.status {
        if let Some(container_statuses) = &status.container_statuses {
            for container_status in container_statuses {
                if let Some(state) = &container_status.last_state {
                    if let Some(terminated) = &state.terminated {
                        if terminated.reason.as_deref() == Some("OOMKilled") {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

async fn create_event(events: &Api<Event>, pod: &Pod) -> anyhow::Result<()> {
    let pod_namespace = ResourceExt::namespace(pod).unwrap_or_default();
    let pod_name = pod.name().unwrap_or_default();
    let pod_uid = ResourceExt::uid(pod).unwrap_or_default();
    let message = format!(
        "Pod {} in namespace {} was OOMKilled.",
        pod_name, pod_namespace
    );
    println!("{}", message);

    let event = Event {
        metadata: kube::api::ObjectMeta {
            name: Some(format!("{}.oomkilled", pod_name)),
            namespace: Some(pod_namespace.to_string()),
            ..Default::default()
        },
        involved_object: ObjectReference {
            kind: Some("Pod".to_string()),
            namespace: Some(pod_namespace.to_string()),
            name: Some(pod_name.to_string()),
            uid: Some(pod_uid),
            api_version: Some("v1".to_string()),
            ..Default::default()
        },
        reason: Some("OOMKilling".to_string()),
        message: Some(message.to_string()),
        type_: Some("Warning".to_string()),
        source: Some(EventSource {
            component: Some("banshee".to_string()),
            ..Default::default()
        }),
        first_timestamp: Some(Time(Utc::now())),
        last_timestamp: Some(Time(Utc::now())),
        count: Some(1),
        ..Default::default()
    };

    let pp = PostParams::default();
    match events.create(&pp, &event).await {
        Ok(_) => println!("Created event for {} in {}", pod_name, pod_namespace),
        Err(e) => eprintln!("Error creating event: {:?}", e),
    }

    Ok(())
}
