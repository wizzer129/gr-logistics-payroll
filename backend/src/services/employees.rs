//! Employee queries. Delete is always a soft delete: loads and paycheck logs reference
//! employees and must keep their history.

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{
    domain::payment_calc::DEFAULT_PAY_RATE_PERCENT,
    models::employee::{CreateEmployee, Employee, UpdateEmployee},
};

macro_rules! columns {
    () => {
        "id, first_name, last_name, dob, email, address, active, pay_rate_percent"
    };
}

pub async fn list_active(pool: &PgPool) -> Result<Vec<Employee>, sqlx::Error> {
    sqlx::query_as(concat!(
        "SELECT ",
        columns!(),
        " FROM employees WHERE active = true ORDER BY last_name, first_name"
    ))
    .fetch_all(pool)
    .await
}

pub async fn create(
    conn: &mut PgConnection,
    input: &CreateEmployee,
    user_id: Uuid,
) -> Result<Employee, sqlx::Error> {
    sqlx::query_as(concat!(
        "INSERT INTO employees
            (first_name, last_name, dob, email, address, pay_rate_percent, created_by, last_edited_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
         RETURNING ",
        columns!()
    ))
    .bind(&input.first_name)
    .bind(&input.last_name)
    .bind(input.dob)
    .bind(&input.email)
    .bind(&input.address)
    .bind(input.pay_rate_percent.unwrap_or(DEFAULT_PAY_RATE_PERCENT))
    .bind(user_id)
    .fetch_one(conn)
    .await
}

/// `None` when no employee has this id.
pub async fn update(
    conn: &mut PgConnection,
    id: Uuid,
    input: &UpdateEmployee,
    user_id: Uuid,
) -> Result<Option<Employee>, sqlx::Error> {
    sqlx::query_as(concat!(
        "UPDATE employees SET
            first_name = COALESCE($2, first_name),
            last_name = COALESCE($3, last_name),
            dob = COALESCE($4, dob),
            email = COALESCE($5, email),
            address = COALESCE($6, address),
            pay_rate_percent = COALESCE($7, pay_rate_percent),
            last_edited_by = $8,
            updated_at = now()
         WHERE id = $1
         RETURNING ",
        columns!()
    ))
    .bind(id)
    .bind(&input.first_name)
    .bind(&input.last_name)
    .bind(input.dob)
    .bind(&input.email)
    .bind(&input.address)
    .bind(input.pay_rate_percent)
    .bind(user_id)
    .fetch_optional(conn)
    .await
}

/// `false` when no employee has this id.
pub async fn soft_delete(
    conn: &mut PgConnection,
    id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE employees SET active = false, last_edited_by = $2, updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(user_id)
    .execute(conn)
    .await?;
    Ok(result.rows_affected() > 0)
}
