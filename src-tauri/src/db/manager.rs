use once_cell::sync::Lazy;
use rusqlite::{Connection, Result, params};
use std::sync::Mutex;

static DB: Lazy<Mutex<Option<Connection>>> = Lazy::new(|| Mutex::new(None));

/// Initialize the SQLite database and run all migrations.
pub fn initialize(path: &str) -> Result<()> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    run_migrations(&conn)?;
    log::info!("Database ready at {}", path);
    let mut db = DB.lock().unwrap();
    *db = Some(conn);
    Ok(())
}

/// Initialize an in-memory database (for testing).
pub fn initialize_memory() -> Result<()> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    run_migrations(&conn)?;
    let mut db = DB.lock().unwrap();
    *db = Some(conn);
    Ok(())
}

/// Get a lock on the database connection.
pub fn with_db<F, T>(f: F) -> Result<T>
where
    F: FnOnce(&Connection) -> Result<T>,
{
    let guard = DB.lock().unwrap();
    let conn = guard.as_ref().expect("Database not initialized");
    f(conn)
}

fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            name TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );"
    )?;

    let migrations: Vec<(&str, &str)> = vec![
        ("v1_cockpit_tables", V1_COCKPIT_TABLES),
        ("v2_metier_skill", V2_METIER_SKILL),
        ("v3_agent_message", V3_AGENT_MESSAGE),
        ("v4_agent_slot_current_task", V4_AGENT_SLOT_TASK),
        ("v5_execution_gate", V5_EXECUTION_GATE),
        ("v6_cost_event", V6_COST_EVENT),
        ("v7_agent_metrics", V7_AGENT_METRICS),
        ("v8_orchestration_record", V8_ORCHESTRATION_RECORD),
    ];

    for (name, sql) in migrations {
        let applied: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM _migrations WHERE name = ?1",
            params![name],
            |row| row.get(0),
        )?;

        if !applied {
            conn.execute_batch(sql)?;
            conn.execute(
                "INSERT INTO _migrations (name) VALUES (?1)",
                params![name],
            )?;
            log::info!("Migration applied: {}", name);
        }
    }

    Ok(())
}

const V1_COCKPIT_TABLES: &str = "
    CREATE TABLE cockpit_session (
        id TEXT PRIMARY KEY NOT NULL,
        projectPath TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'idle',
        chairmanBrief TEXT,
        createdAt TEXT NOT NULL,
        updatedAt TEXT NOT NULL
    );
    CREATE UNIQUE INDEX idx_cockpit_session_project_active
        ON cockpit_session (projectPath, status)
        WHERE status != 'closed';

    CREATE TABLE agent_slot (
        id TEXT PRIMARY KEY NOT NULL,
        cockpitSessionId TEXT NOT NULL REFERENCES cockpit_session(id) ON DELETE CASCADE,
        slotIndex INTEGER NOT NULL,
        status TEXT NOT NULL DEFAULT 'empty',
        agentType TEXT NOT NULL,
        worktreePath TEXT,
        branchName TEXT,
        skillId TEXT,
        createdAt TEXT NOT NULL,
        updatedAt TEXT NOT NULL
    );
    CREATE INDEX idx_agent_slot_slot_index ON agent_slot (slotIndex);
    CREATE INDEX idx_agent_slot_status ON agent_slot (status);
";

const V2_METIER_SKILL: &str = "
    CREATE TABLE metier_skill (
        id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL,
        family TEXT NOT NULL,
        skillMdPath TEXT NOT NULL,
        requiredMcps TEXT,
        description TEXT,
        createdAt TEXT NOT NULL
    );
    CREATE UNIQUE INDEX idx_metier_skill_name ON metier_skill (name);
    CREATE INDEX idx_metier_skill_family ON metier_skill (family);
";

const V3_AGENT_MESSAGE: &str = "
    CREATE TABLE agent_message (
        id TEXT PRIMARY KEY NOT NULL,
        content TEXT NOT NULL,
        messageType TEXT NOT NULL,
        fromSlotId TEXT NOT NULL REFERENCES agent_slot(id) ON DELETE CASCADE,
        toSlotId TEXT REFERENCES agent_slot(id) ON DELETE CASCADE,
        isBroadcast INTEGER NOT NULL DEFAULT 0,
        readAt TEXT,
        createdAt TEXT NOT NULL
    );
    CREATE INDEX idx_agent_message_from_slot ON agent_message (fromSlotId);
    CREATE INDEX idx_agent_message_to_slot ON agent_message (toSlotId);
    CREATE INDEX idx_agent_message_created_at ON agent_message (createdAt);
";

const V4_AGENT_SLOT_TASK: &str = "
    ALTER TABLE agent_slot ADD COLUMN currentTask TEXT;
";

const V5_EXECUTION_GATE: &str = "
    CREATE TABLE execution_gate (
        id TEXT PRIMARY KEY NOT NULL,
        agentSlotId TEXT NOT NULL REFERENCES agent_slot(id) ON DELETE CASCADE,
        status TEXT NOT NULL DEFAULT 'pending',
        operationType TEXT NOT NULL,
        operationPayload TEXT NOT NULL,
        riskLevel TEXT NOT NULL,
        estimatedImpact TEXT,
        approvedBy TEXT,
        approvedAt TEXT,
        deniedReason TEXT,
        rollbackPayload TEXT,
        auditEntry TEXT,
        createdAt TEXT NOT NULL,
        updatedAt TEXT NOT NULL
    );
    CREATE INDEX idx_execution_gate_status ON execution_gate (status);
    CREATE INDEX idx_execution_gate_risk_level ON execution_gate (riskLevel);
    CREATE INDEX idx_execution_gate_created_at ON execution_gate (createdAt);
";

const V6_COST_EVENT: &str = "
    CREATE TABLE cost_event (
        id TEXT PRIMARY KEY NOT NULL,
        agentSlotId TEXT NOT NULL REFERENCES agent_slot(id) ON DELETE CASCADE,
        provider TEXT NOT NULL,
        model TEXT NOT NULL,
        inputTokens INTEGER NOT NULL,
        outputTokens INTEGER NOT NULL,
        costCents INTEGER NOT NULL DEFAULT 0,
        createdAt TEXT NOT NULL
    );
    CREATE INDEX idx_cost_event_slot ON cost_event (agentSlotId);
    CREATE INDEX idx_cost_event_created_at ON cost_event (createdAt);
";

const V7_AGENT_METRICS: &str = "
    CREATE TABLE agent_metrics (
        id TEXT PRIMARY KEY NOT NULL,
        agentSlotId TEXT NOT NULL REFERENCES agent_slot(id) ON DELETE CASCADE,
        totalStoriesCompleted INTEGER NOT NULL DEFAULT 0,
        totalStoriesFailed INTEGER NOT NULL DEFAULT 0,
        avgStoryTimeMs INTEGER NOT NULL DEFAULT 0,
        conflictsEncountered INTEGER NOT NULL DEFAULT 0,
        failoverAttempts INTEGER NOT NULL DEFAULT 0,
        lastStoryStartedAt TEXT,
        updatedAt TEXT NOT NULL
    );
    CREATE UNIQUE INDEX idx_agent_metrics_slot ON agent_metrics (agentSlotId);
";

const V8_ORCHESTRATION_RECORD: &str = "
    CREATE TABLE orchestration_record (
        id TEXT PRIMARY KEY NOT NULL,
        cockpitSessionId TEXT NOT NULL REFERENCES cockpit_session(id) ON DELETE CASCADE,
        prdPath TEXT NOT NULL,
        prdName TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'pending',
        totalStories INTEGER NOT NULL DEFAULT 0,
        completedStories INTEGER NOT NULL DEFAULT 0,
        failedStories INTEGER NOT NULL DEFAULT 0,
        currentLayer INTEGER NOT NULL DEFAULT 0,
        layersJson TEXT,
        resultSummary TEXT,
        mergedBranches TEXT,
        conflicts TEXT,
        totalCostCents INTEGER NOT NULL DEFAULT 0,
        startedAt TEXT NOT NULL,
        finishedAt TEXT,
        updatedAt TEXT NOT NULL
    );
    CREATE INDEX idx_orch_record_session ON orchestration_record (cockpitSessionId);
    CREATE INDEX idx_orch_record_status ON orchestration_record (status);
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_run_on_fresh_db() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run_migrations(&conn).unwrap();

        // Verify all 6 migrations applied
        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 8);
    }

    #[test]
    fn test_tables_exist_with_correct_columns() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run_migrations(&conn).unwrap();

        let tables = vec![
            "cockpit_session",
            "agent_slot",
            "metier_skill",
            "agent_message",
            "execution_gate",
            "cost_event",
            "agent_metrics",
            "orchestration_record",
        ];

        for table in tables {
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name=?1",
                    params![table],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(exists, "Table {} should exist", table);
        }
    }

    #[test]
    fn test_migrations_are_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run_migrations(&conn).unwrap();
        // Running again should not fail
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 8);
    }

    #[test]
    fn test_foreign_key_cascade() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run_migrations(&conn).unwrap();

        // Insert session + slot + cost_event
        conn.execute(
            "INSERT INTO cockpit_session (id, projectPath, status, createdAt, updatedAt)
             VALUES ('s1', '/tmp', 'active', datetime('now'), datetime('now'))",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO agent_slot (id, cockpitSessionId, slotIndex, status, agentType, createdAt, updatedAt)
             VALUES ('a1', 's1', 0, 'running', 'claude', datetime('now'), datetime('now'))",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO cost_event (id, agentSlotId, provider, model, inputTokens, outputTokens, costCents, createdAt)
             VALUES ('c1', 'a1', 'anthropic', 'sonnet', 1000, 500, 10, datetime('now'))",
            [],
        ).unwrap();

        // Delete session — should cascade to slot and cost_event
        conn.execute("DELETE FROM cockpit_session WHERE id = 's1'", []).unwrap();

        let slot_count: i32 = conn.query_row("SELECT COUNT(*) FROM agent_slot", [], |r| r.get(0)).unwrap();
        let cost_count: i32 = conn.query_row("SELECT COUNT(*) FROM cost_event", [], |r| r.get(0)).unwrap();
        assert_eq!(slot_count, 0, "Slots should be cascade deleted");
        assert_eq!(cost_count, 0, "Cost events should be cascade deleted");
    }
}
