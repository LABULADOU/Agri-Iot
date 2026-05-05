use yew::prelude::*;
use yew_router::prelude::*;

mod components;
mod pages;
mod api;
mod types;

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/")]
    Dashboard,
    #[at("/devices")]
    DeviceList,
    #[at("/devices/:id")]
    DeviceDetail { id: String },
    #[at("/rules")]
    RuleList,
    #[at("/alerts")]
    Alerts,
    #[at("/settings")]
    Settings,
    #[not_found]
    #[at("/404")]
    NotFound,
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::Dashboard => html! { <pages::dashboard::Dashboard /> },
        Route::DeviceList => html! { <pages::device_list::DeviceList /> },
        Route::DeviceDetail { id } => html! { <pages::device_detail::DeviceDetail id={id} /> },
        Route::RuleList => html! { <pages::rule_list::RuleList /> },
        Route::Alerts => html! { <pages::alerts::Alerts /> },
        Route::Settings => html! { <pages::settings::Settings /> },
        Route::NotFound => html! { <h1>{"404 Not Found"}</h1> },
    }
}

#[function_component(App)]
fn app() -> Html {
    html! {
        <BrowserRouter>
            <div class="app">
                <components::sidebar::Sidebar />
                <main class="main-content">
                    <Switch<Route> render={switch} />
                </main>
            </div>
        </BrowserRouter>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
