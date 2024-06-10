use futures::{pin_mut, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, ResourceExt},
    runtime::{
        events::{Event, EventType, Recorder, Reporter},
        reflector::Lookup,
        watcher, WatchStreamExt,
    },
    Client, Resource,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::all(client.clone());

    let config = kube::runtime::watcher::Config::default();
    let stream = watcher(pods, config).applied_objects();
    pin_mut!(stream);

    while let Some(pod) = stream.try_next().await? {
        if let Some(restart_count) = was_killed(&pod) {
            create_event(&client, &pod, restart_count).await?;
        }
    }

    Ok(())
}

fn was_killed(pod: &Pod) -> Option<i32> {
    if let Some(status) = &pod.status {
        if let Some(container_statuses) = &status.container_statuses {
            for container_status in container_statuses {
                if let Some(state) = &container_status.last_state {
                    if let Some(terminated) = &state.terminated {
                        if terminated.reason.as_deref() == Some("OOMKilled") {
                            return Some(container_status.restart_count);
                        }
                    }
                }
            }
        }
    }
    None
}

async fn create_event(client: &Client, pod: &Pod, restart_count: i32) -> anyhow::Result<()> {
    let pod_namespace = ResourceExt::namespace(pod).unwrap_or_default();
    let pod_name = pod.name().unwrap_or_default();
    let pod_ref = pod.object_ref(&());

    let message = format!(
        "Pod {} in namespace {} was OOMKilled on restart {}.",
        pod_name, pod_namespace, restart_count
    );
    println!("{}", message);

    let reporter = Reporter {
        controller: "banshee".into(),
        instance: None,
    };

    let recorder = Recorder::new(client.clone(), reporter, pod_ref);
    recorder
        .publish(Event {
            action: "Terminated".into(),
            reason: "OOMKilling".into(),
            note: Some(message.to_string()),
            type_: EventType::Warning,
            secondary: None,
        })
        .await?;

    Ok(())
}
