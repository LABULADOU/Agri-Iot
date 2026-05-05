use yew::prelude::*;
use yew_router::prelude::*;
use crate::Route;

#[function_component(Sidebar)]
pub fn sidebar() -> Html {
    let nav_items = vec![
        ("仪表盘", Route::Dashboard),
        ("设备管理", Route::DeviceList),
        ("规则引擎", Route::RuleList),
        ("告警记录", Route::Alerts),
        ("系统设置", Route::Settings),
    ];

    html! {
        <nav class="sidebar">
            <h2 style="padding: 0 24px 20px; color: #fff;">{"农业物联网"}</h2>
            {for nav_items.into_iter().map(|(label, route)| {
                html! {
                    <Link<Route> to={route}>
                        {label}
                    </Link<Route>>
                }
            })}
        </nav>
    }
}
