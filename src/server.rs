use anyhow::{Error, Result};

use axum::{
    Router,
    extract::{Query, State},
    http::{
        StatusCode, Uri,
        header::{CACHE_CONTROL, CONTENT_TYPE},
        uri::PathAndQuery,
    },
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    serve,
};

use const_format::concatcp;
use handlebars::Handlebars;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;

use crate::{
    model::{ConfigPoint, Diff, SystemProperty},
    service::Service,
    utils::get_var,
};

const BASE: &str = "/conf-diff";

#[derive(Clone)]
struct AppContext<S>
where
    S: Service,
{
    service: S,
    handlebars: Handlebars<'static>,
}

impl<S> AppContext<S>
where
    S: Service,
{
    fn new(service: S) -> Result<Self> {
        #[derive(RustEmbed)]
        #[folder = "web/templates"]
        #[include = "*.hbs"]
        struct Templates;

        let mut handlebars = Handlebars::new();

        #[cfg(debug_assertions)]
        handlebars.set_dev_mode(true);
        handlebars.set_strict_mode(true);

        handlebars.register_embed_templates_with_extension::<Templates>(".hbs")?;
        handlebars.register_template_string("VERSION", env!("CARGO_PKG_VERSION"))?;

        Ok(Self {
            service,
            handlebars,
        })
    }
}

struct AppError(Error);

impl<E> From<E> for AppError
where
    E: Into<Error>,
{
    fn from(error: E) -> Self {
        Self(error.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()).into_response()
    }
}

#[derive(Deserialize)]
struct IndexInput {
    left: Option<String>,
    right: Option<String>,
}

async fn get_asset(uri: Uri) -> Response {
    #[derive(RustEmbed)]
    #[folder = "web/dist"]
    #[include = "*.css"]
    #[include = "*.js"]
    #[cfg_attr(debug_assertions, include = "*.map")]
    struct Dist;

    #[derive(RustEmbed)]
    #[folder = "web/static"]
    struct Static;

    let path = uri.path();

    if !path.starts_with(concatcp!(BASE, '/')) {
        return Redirect::permanent(&format!(
            "{BASE}{}",
            uri.path_and_query()
                .map(PathAndQuery::as_str)
                .unwrap_or_default()
        ))
        .into_response();
    }

    let path = path.trim_start_matches(concatcp!(BASE, '/'));

    if let Some(asset) = Dist::get(path) {
        (
            [
                (CACHE_CONTROL, "public, max-age=31536000, immutable"),
                (CONTENT_TYPE, asset.metadata.mimetype()),
            ],
            asset.data,
        )
            .into_response()
    } else if let Some(asset) = Static::get(path) {
        ([(CONTENT_TYPE, asset.metadata.mimetype())], asset.data).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn get_index<S>(
    State(app_context): State<AppContext<S>>,
    Query(query): Query<IndexInput>,
) -> Result<impl IntoResponse, AppError>
where
    S: Service,
{
    #[derive(Serialize)]
    struct ConfigPointDifference {
        name: String,
        organization: Option<String>,
        left_environment_config_point: Option<ConfigPoint>,
        right_environment_config_point: Option<ConfigPoint>,
    }

    #[derive(Serialize)]
    struct Model {
        selected_left_environment: Option<String>,
        selected_right_environment: Option<String>,
        environment_names: Vec<String>,
        controller_error: Option<String>,
        config_point_differences: Vec<ConfigPointDifference>,
        config_points_error: Option<String>,
        system_property_differences: Vec<SystemPropertyDifference>,
        system_properties_error: Option<String>,
    }

    #[derive(Serialize)]
    struct SystemPropertyDifference {
        name: String,
        left_environment_system_property: Option<SystemProperty>,
        right_environment_system_property: Option<SystemProperty>,
    }

    let mut model = Model {
        selected_left_environment: None,
        selected_right_environment: None,
        environment_names: Vec::new(),
        controller_error: None,
        config_point_differences: Vec::new(),
        config_points_error: None,
        system_property_differences: Vec::new(),
        system_properties_error: None,
    };

    if let Some(left_environment_name) = &query.left
        && let Some(right_environment_name) = &query.right
    {
        match app_context
            .service
            .get_environment_config_diffs(left_environment_name, right_environment_name)
        {
            Ok(environment_config_diffs) => {
                model.environment_names = environment_config_diffs.environment_names;

                match environment_config_diffs.config_point_diffs {
                    Ok(config_point_differences) => {
                        for config_point_difference in config_point_differences {
                            model.config_point_differences.push(ConfigPointDifference {
                                name: config_point_difference.item().name.clone(),
                                organization: config_point_difference.item().organization.clone(),
                                left_environment_config_point: config_point_difference.left_item,
                                right_environment_config_point: config_point_difference.right_item,
                            });
                        }
                    }
                    Err(error) => model.config_points_error = Some(error.to_string()),
                }

                match environment_config_diffs.system_property_diffs {
                    Ok(system_property_differences) => {
                        for system_property_difference in system_property_differences {
                            model
                                .system_property_differences
                                .push(SystemPropertyDifference {
                                    name: system_property_difference.item().name.clone(),
                                    left_environment_system_property: system_property_difference
                                        .left_item,
                                    right_environment_system_property: system_property_difference
                                        .right_item,
                                });
                        }
                    }
                    Err(error) => model.config_points_error = Some(error.to_string()),
                }
            }
            Err(error) => model.controller_error = Some(error.to_string()),
        }
    } else {
        match app_context.service.get_environment_names() {
            Ok(environment_names) => model.environment_names = environment_names,
            Err(error) => model.controller_error = Some(error.to_string()),
        }
    }

    model.selected_left_environment = query.left;
    model.selected_right_environment = query.right;

    Ok(Html(app_context.handlebars.render("index", &model)?))
}

pub async fn start<S>(service: S) -> Result<()>
where
    S: Service + 'static,
{
    let app_context = AppContext::new(service)?;

    let router = Router::new()
        .route(concatcp!(BASE, '/'), get(get_index::<S>))
        .fallback(get(get_asset))
        .with_state(app_context)
        .layer(ServiceBuilder::new().layer(CompressionLayer::new()));

    let listener = TcpListener::bind(get_var("LISTENER")?).await?;

    serve(listener, router).await.map_err(Into::into)
}
