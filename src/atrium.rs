use rusqlite::Connection;

#[derive(Debug)]
pub enum AtriumError {
    DbOpen(String),
    Query(String),
}

impl std::fmt::Display for AtriumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AtriumError::DbOpen(msg) => write!(f, "Failed to open Atrium DB: {msg}"),
            AtriumError::Query(msg) => write!(f, "Atrium query failed: {msg}"),
        }
    }
}

fn open_readonly(db_path: &str) -> Result<Connection, AtriumError> {
    let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
        | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX;
    Connection::open_with_flags(db_path, flags)
        .map_err(|e| AtriumError::DbOpen(e.to_string()))
}

/// Appointments with their attribution data and patient/treatment info.
pub fn patient_attribution(
    db_path: &str,
    date_from: &str,
    date_to: &str,
) -> Result<serde_json::Value, AtriumError> {
    let conn = open_readonly(db_path)?;

    let mut stmt = conn.prepare(
        "SELECT
            a.id, a.date, a.start_time, a.status, a.created_at,
            p.first_name, p.last_name, p.phone, p.email,
            t.name AS treatment, t.category, t.price_pence,
            attr.source, attr.medium, attr.campaign, attr.landing_page, attr.referrer
         FROM appointments a
         JOIN patients p ON p.id = a.patient_id
         JOIN treatments t ON t.id = a.treatment_id
         LEFT JOIN attribution attr ON attr.appointment_id = a.id
         WHERE a.date BETWEEN ?1 AND ?2
         ORDER BY a.date, a.start_time"
    ).map_err(|e| AtriumError::Query(e.to_string()))?;

    let rows = stmt.query_map(rusqlite::params![date_from, date_to], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "date": row.get::<_, String>(1)?,
            "time": row.get::<_, String>(2)?,
            "status": row.get::<_, String>(3)?,
            "created_at": row.get::<_, String>(4)?,
            "patient_first_name": row.get::<_, String>(5)?,
            "patient_last_name": row.get::<_, String>(6)?,
            "patient_phone": row.get::<_, String>(7)?,
            "patient_email": row.get::<_, Option<String>>(8)?,
            "treatment": row.get::<_, String>(9)?,
            "category": row.get::<_, String>(10)?,
            "price_pence": row.get::<_, Option<i64>>(11)?,
            "source": row.get::<_, Option<String>>(12)?,
            "medium": row.get::<_, Option<String>>(13)?,
            "campaign": row.get::<_, Option<String>>(14)?,
            "landing_page": row.get::<_, Option<String>>(15)?,
            "referrer": row.get::<_, Option<String>>(16)?,
        }))
    }).map_err(|e| AtriumError::Query(e.to_string()))?;

    let appointments: Vec<serde_json::Value> = rows
        .filter_map(|r| r.ok())
        .collect();

    Ok(serde_json::json!({
        "total": appointments.len(),
        "appointments": appointments
    }))
}

/// Confirmed bookings aggregated by attribution source/medium.
pub fn channel_breakdown(
    db_path: &str,
    date_from: &str,
    date_to: &str,
) -> Result<serde_json::Value, AtriumError> {
    let conn = open_readonly(db_path)?;

    let mut stmt = conn.prepare(
        "SELECT
            COALESCE(attr.source, '(direct)') AS source,
            COALESCE(attr.medium, '(none)') AS medium,
            COUNT(*) AS total_bookings,
            SUM(CASE WHEN a.status = 'confirmed' THEN 1 ELSE 0 END) AS confirmed,
            SUM(CASE WHEN a.status = 'cancelled' THEN 1 ELSE 0 END) AS cancelled,
            SUM(COALESCE(t.price_pence, 0)) AS total_revenue_pence
         FROM appointments a
         JOIN treatments t ON t.id = a.treatment_id
         LEFT JOIN attribution attr ON attr.appointment_id = a.id
         WHERE a.date BETWEEN ?1 AND ?2
         GROUP BY source, medium
         ORDER BY total_bookings DESC"
    ).map_err(|e| AtriumError::Query(e.to_string()))?;

    let rows = stmt.query_map(rusqlite::params![date_from, date_to], |row| {
        Ok(serde_json::json!({
            "source": row.get::<_, String>(0)?,
            "medium": row.get::<_, String>(1)?,
            "total_bookings": row.get::<_, i64>(2)?,
            "confirmed": row.get::<_, i64>(3)?,
            "cancelled": row.get::<_, i64>(4)?,
            "total_revenue_pence": row.get::<_, i64>(5)?,
        }))
    }).map_err(|e| AtriumError::Query(e.to_string()))?;

    let channels: Vec<serde_json::Value> = rows
        .filter_map(|r| r.ok())
        .collect();

    Ok(serde_json::json!({
        "channels": channels
    }))
}

/// Lead pipeline overview — stages, sources, and activity counts.
pub fn lead_pipeline(
    db_path: &str,
    date_from: &str,
    date_to: &str,
) -> Result<serde_json::Value, AtriumError> {
    let conn = open_readonly(db_path)?;

    let mut stmt = conn.prepare(
        "SELECT
            l.id, l.name, l.phone, l.email, l.stage, l.source,
            l.treatment_interest, l.next_action_date,
            l.created_at, l.updated_at,
            (SELECT COUNT(*) FROM lead_activity la WHERE la.lead_id = l.id) AS activity_count
         FROM leads l
         WHERE date(l.created_at) BETWEEN ?1 AND ?2
         ORDER BY l.created_at DESC"
    ).map_err(|e| AtriumError::Query(e.to_string()))?;

    let rows = stmt.query_map(rusqlite::params![date_from, date_to], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "name": row.get::<_, String>(1)?,
            "phone": row.get::<_, String>(2)?,
            "email": row.get::<_, Option<String>>(3)?,
            "stage": row.get::<_, String>(4)?,
            "source": row.get::<_, String>(5)?,
            "treatment_interest": row.get::<_, Option<String>>(6)?,
            "next_action_date": row.get::<_, Option<String>>(7)?,
            "created_at": row.get::<_, String>(8)?,
            "updated_at": row.get::<_, String>(9)?,
            "activity_count": row.get::<_, i64>(10)?,
        }))
    }).map_err(|e| AtriumError::Query(e.to_string()))?;

    let leads: Vec<serde_json::Value> = rows
        .filter_map(|r| r.ok())
        .collect();

    Ok(serde_json::json!({
        "total": leads.len(),
        "leads": leads
    }))
}
