use yew::prelude::*;
use crate::{api, types::SensorReading};

#[derive(Properties, PartialEq)]
pub struct Props {
    pub id: String,
}

#[function_component(DeviceDetail)]
pub fn device_detail(props: &Props) -> Html {
    let readings = use_state(Vec::new);
    let id = props.id.clone();

    {
        let readings = readings.clone();
        let id = id.clone();
        use_effect_with((), move |_| {
            let id = id.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let path = format!("/devices/{}/readings?limit=100", id);
                match api::get::<Vec<SensorReading>>(&path).await {
                    Ok(data) => readings.set(data),
                    Err(e) => web_sys::console::error_1(&format!("Failed to load readings: {}", e).into()),
                }
            });
            || ()
        });
    }

    html! {
        <div>
            <h1>{"设备详情: "}{&id}</h1>
            <div class="card">
                <h3>{"历史数据"}</h3>
                <table>
                    <thead>
                        <tr>
                            <th>{"指标"}</th>
                            <th>{"数值"}</th>
                            <th>{"时间"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for readings.iter().map(|r| {
                            let time = chrono::DateTime::from_timestamp(r.timestamp, 0)
                                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                .unwrap_or_default();
                            html! {
                                <tr>
                                    <td>{&r.metric}</td>
                                    <td>{format!("{} {}", r.value, r.unit)}</td>
                                    <td>{time}</td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
            </div>
        </div>
    }
}
