use yew::prelude::*;
use crate::api;
use serde_json::Value;

#[function_component(Dashboard)]
pub fn dashboard() -> Html {
    let summary = use_state(|| None);

    {
        let summary = summary.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                match api::get::<Value>("/dashboard/summary").await {
                    Ok(data) => summary.set(Some(data)),
                    Err(e) => web_sys::console::error_1(&format!("Failed to load summary: {}", e).into()),
                }
            });
            || ()
        });
    }

    html! {
        <div>
            <h1>{"仪表盘"}</h1>
            <div class="card">
                {if let Some(data) = (*summary).clone() {
                    html! {
                        <div>
                            <div class="metric-card">
                                <div class="metric-value">{data["total_devices"].as_i64().unwrap_or(0)}</div>
                                <div class="metric-label">{"设备总数"}</div>
                            </div>
                            <div class="metric-card">
                                <div class="metric-value" style="color: #28a745;">{data["online_devices"].as_i64().unwrap_or(0)}</div>
                                <div class="metric-label">{"在线设备"}</div>
                            </div>
                            <div class="metric-card">
                                <div class="metric-value" style="color: #ffc107;">{data["active_rules"].as_i64().unwrap_or(0)}</div>
                                <div class="metric-label">{"活跃规则"}</div>
                            </div>
                        </div>
                    }
                } else {
                    html! { <p>{"加载中..."}</p> }
                }}
            </div>
        </div>
    }
}
