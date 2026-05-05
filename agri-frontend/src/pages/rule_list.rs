use yew::prelude::*;
use crate::{api, types::Rule};

#[function_component(RuleList)]
pub fn rule_list() -> Html {
    let rules = use_state(Vec::new);

    {
        let rules = rules.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                match api::get::<Vec<Rule>>("/rules").await {
                    Ok(data) => rules.set(data),
                    Err(e) => web_sys::console::error_1(&format!("Failed to load rules: {}", e).into()),
                }
            });
            || ()
        });
    }

    html! {
        <div>
            <h1>{"规则引擎"}</h1>
            <div class="card">
                <table>
                    <thead>
                        <tr>
                            <th>{"名称"}</th>
                            <th>{"类型"}</th>
                            <th>{"状态"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {for rules.iter().map(|r| {
                            html! {
                                <tr>
                                    <td>{&r.name}</td>
                                    <td>{&r.trigger_type}</td>
                                    <td>{if r.enabled {"启用"} else {"禁用"}}</td>
                                </tr>
                            }
                        })}
                    </tbody>
                </table>
            </div>
        </div>
    }
}
