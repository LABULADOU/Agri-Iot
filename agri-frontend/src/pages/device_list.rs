use yew::prelude::*;
use crate::{api, types::Device};

#[function_component(DeviceList)]
pub fn device_list() -> Html {
    let devices = use_state(Vec::new);

    {
        let devices = devices.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                match api::get::<Vec<Device>>("/devices").await {
                    Ok(data) => devices.set(data),
                    Err(e) => web_sys::console::error_1(&format!("Failed to load devices: {}", e).into()),
                }
            });
            || ()
        });
    }

    html! {
        <div>
            <h1>{"设备管理"}</h1>
            <div class="card">
                <table>
                    <thead>
                        <tr>
                            <th>{"名称"}</th>
                            <th>{"节点 ID"}</th>
                            <th>{"类型"}</th>
                            <th>{"状态"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for devices.iter().map(|d| {
                            let status_class = match d.status.as_str() {
                                "online" => "status-online",
                                "offline" => "status-offline",
                                _ => "status-error",
                            };
                            html! {
                                <tr>
                                    <td>{&d.name}</td>
                                    <td>{&d.node_id}</td>
                                    <td>{&d.device_type}</td>
                                    <td class={status_class}>{&d.status}</td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
            </div>
        </div>
    }
}
