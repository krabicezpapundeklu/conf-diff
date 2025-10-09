use anyhow::{Context, Result};
use dotenvy::var;
use oracle::RowValue;

use crate::model::{ConfigPoint, SystemProperty};

struct ConnectionImpl(oracle::Connection);

pub trait Repository {
    type Connection;

    fn get_config_points(
        &self,
        environment_connection: &Self::Connection,
    ) -> Result<Vec<ConfigPoint>>;
    fn get_controller_connection(&self) -> Result<Self::Connection>;

    fn get_environment_connection(
        &self,
        controller_connection: &Self::Connection,
        environment_name: &str,
    ) -> Result<Self::Connection>;

    fn get_environment_names(&self) -> Result<Vec<String>>;

    fn get_system_properties(
        &self,
        controller_connection: &Self::Connection,
        environment_name: &str,
    ) -> Result<Vec<SystemProperty>>;
}

struct RepositoryImpl {
    controller_connection_string: String,
    controller_schema: String,
    controller_user: String,
    controller_password: String,
    environment_user: String,
    environment_password: String,
    environment_schema_suffix: String,
}

impl Repository for RepositoryImpl {
    type Connection = ConnectionImpl;

    fn get_config_points(
        &self,
        environment_connection: &Self::Connection,
    ) -> Result<Vec<ConfigPoint>> {
        let rows = environment_connection
            .0
            .query_as::<(String, Option<String>, Option<String>)>(
                "SELECT cp.config_name, cp.config_value, o.org_desc
                FROM config_point cp
                LEFT JOIN um_organization o ON o.org_id = cp.org_id",
                &[],
            )?;

        let mut config_points = Vec::new();

        for row in rows {
            let row = row?;

            config_points.push(ConfigPoint {
                name: row.0,
                value: row.1,
                organization: row.2,
            });
        }

        Ok(config_points)
    }

    fn get_controller_connection(&self) -> Result<Self::Connection> {
        let connection = oracle::Connection::connect(
            &self.controller_user,
            &self.controller_password,
            &self.controller_connection_string,
        )?;

        connection.set_current_schema(&self.controller_schema)?;

        Ok(ConnectionImpl(connection))
    }

    fn get_environment_connection(
        &self,
        controller_connection: &Self::Connection,
        environment_name: &str,
    ) -> Result<Self::Connection> {
        #[derive(RowValue)]
        struct EnvironmentConnectionInfo {
            server_name: String,
            sid: String,
            port: i64,
            schema: String,
        }

        let environment_connection_info = controller_connection.0.query_row_as::<EnvironmentConnectionInfo>(
            "SELECT s.system_name, ad.server_name, ad.sid, ad.port, sp.property_value schema
            FROM system s
            JOIN app_database ad ON ad.database_id = s.database_id
            JOIN system_properties sp ON sp.system_id = s.system_id AND sp.property_name = 'HM_DB_PREFIX'
            WHERE LOWER(s.system_name) = LOWER(:1)", &[&environment_name])?;

        let connection = oracle::Connection::connect(
            &self.environment_user,
            &self.environment_password,
            format!(
                "{}:{}/{}",
                environment_connection_info.server_name,
                environment_connection_info.port,
                environment_connection_info.sid
            ),
        )?;

        connection.set_current_schema(&format!(
            "{}{}",
            &environment_connection_info.schema, &self.environment_schema_suffix
        ))?;

        Ok(ConnectionImpl(connection))
    }

    fn get_environment_names(&self) -> Result<Vec<String>> {
        let controller_connection = self.get_controller_connection()?;

        let rows = controller_connection.0.query_as::<String>(
            "SELECT s.system_name
            FROM system s
            JOIN system_properties sp ON sp.system_id = s.system_id AND sp.property_name = 'HM_DB_PREFIX'
            ORDER BY LOWER(s.system_name)", &[])?;

        let mut environment_names = Vec::new();

        for row in rows {
            environment_names.push(row?);
        }

        Ok(environment_names)
    }

    fn get_system_properties(
        &self,
        controller_connection: &Self::Connection,
        environment_name: &str,
    ) -> Result<Vec<SystemProperty>> {
        let rows = controller_connection
            .0
            .query_as::<(String, Option<String>)>(
                "SELECT sp.property_name, sp.property_value
                FROM system_properties sp
                JOIN system s ON s.system_id = sp.system_id
                WHERE LOWER(s.system_name) = LOWER(:1)",
                &[&environment_name],
            )?;

        let mut system_properties = Vec::new();

        for row in rows {
            let row = row?;

            system_properties.push(SystemProperty {
                name: row.0,
                value: row.1,
            });
        }

        Ok(system_properties)
    }
}

fn get_var(key: &str) -> Result<String> {
    var(key).context(format!("{key} is not set"))
}

pub fn new_repository() -> Result<impl Repository> {
    Ok(RepositoryImpl {
        controller_connection_string: get_var("CONTROLLER_CONNECTION_STRING")?,
        controller_schema: get_var("CONTROLLER_SCHEMA")?,
        controller_user: get_var("CONTROLLER_USER")?,
        controller_password: get_var("CONTROLLER_PASSWORD")?,
        environment_user: get_var("ENVIRONMENT_USER")?,
        environment_password: get_var("ENVIRONMENT_PASSWORD")?,
        environment_schema_suffix: get_var("ENVIRONMENT_SCHEMA_SUFFIX")?,
    })
}
