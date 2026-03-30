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
        ("v9_org_role", V9_ORG_ROLE),
        ("v10_budget", V10_BUDGET),
        ("v11_heartbeat", V11_HEARTBEAT),
        ("v12_workspace", V12_WORKSPACE),
        ("v13_agent_runtime", V13_AGENT_RUNTIME),
        ("v14_config_snapshot", V14_CONFIG_SNAPSHOT),
        ("v15_learning", V15_LEARNING),
        ("v16_agent_memory", V16_AGENT_MEMORY),
        ("v17_trust_score", V17_TRUST_SCORE),
        ("v18_claude_session_id", V18_CLAUDE_SESSION_ID),
        ("v19_chat_history", V19_CHAT_HISTORY),
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

const V9_ORG_ROLE: &str = "
    CREATE TABLE org_role (
        id TEXT PRIMARY KEY NOT NULL,
        sessionId TEXT NOT NULL REFERENCES cockpit_session(id) ON DELETE CASCADE,
        name TEXT NOT NULL,
        roleType TEXT NOT NULL DEFAULT 'custom',
        parentRoleId TEXT REFERENCES org_role(id) ON DELETE SET NULL,
        assignedSlotId TEXT REFERENCES agent_slot(id) ON DELETE SET NULL,
        goalDescription TEXT,
        skillNames TEXT,
        authority TEXT NOT NULL DEFAULT 'limited',
        createdAt TEXT NOT NULL,
        updatedAt TEXT NOT NULL
    );
    CREATE INDEX idx_org_role_session ON org_role (sessionId);
    CREATE INDEX idx_org_role_parent ON org_role (parentRoleId);
";

const V10_BUDGET: &str = "
    CREATE TABLE budget_config (
        id TEXT PRIMARY KEY NOT NULL,
        sessionId TEXT NOT NULL REFERENCES cockpit_session(id) ON DELETE CASCADE,
        slotId TEXT REFERENCES agent_slot(id) ON DELETE CASCADE,
        budgetCents INTEGER NOT NULL DEFAULT 2500,
        warningThresholdPct INTEGER NOT NULL DEFAULT 80,
        hardStopEnabled INTEGER NOT NULL DEFAULT 1,
        throttleEnabled INTEGER NOT NULL DEFAULT 1,
        dailyLimitCents INTEGER,
        perStoryLimitCents INTEGER,
        createdAt TEXT NOT NULL,
        updatedAt TEXT NOT NULL
    );
    CREATE INDEX idx_budget_config_session ON budget_config (sessionId);
    CREATE INDEX idx_budget_config_slot ON budget_config (slotId);

    CREATE TABLE budget_alert (
        id TEXT PRIMARY KEY NOT NULL,
        budgetConfigId TEXT NOT NULL REFERENCES budget_config(id) ON DELETE CASCADE,
        alertType TEXT NOT NULL,
        currentSpendCents INTEGER NOT NULL,
        budgetCents INTEGER NOT NULL,
        percentUsed REAL NOT NULL,
        message TEXT NOT NULL,
        acknowledged INTEGER NOT NULL DEFAULT 0,
        createdAt TEXT NOT NULL
    );
    CREATE INDEX idx_budget_alert_config ON budget_alert (budgetConfigId);
";

const V11_HEARTBEAT: &str = "
    CREATE TABLE heartbeat_config (
        id TEXT PRIMARY KEY NOT NULL,
        sessionId TEXT NOT NULL REFERENCES cockpit_session(id) ON DELETE CASCADE,
        slotId TEXT REFERENCES agent_slot(id) ON DELETE CASCADE,
        intervalMs INTEGER NOT NULL DEFAULT 30000,
        enabled INTEGER NOT NULL DEFAULT 1,
        lastPulseAt TEXT,
        lastPulseResult TEXT,
        consecutiveFailures INTEGER NOT NULL DEFAULT 0,
        maxFailures INTEGER NOT NULL DEFAULT 5,
        createdAt TEXT NOT NULL,
        updatedAt TEXT NOT NULL
    );
    CREATE INDEX idx_heartbeat_session ON heartbeat_config (sessionId);

    CREATE TABLE scheduled_run (
        id TEXT PRIMARY KEY NOT NULL,
        projectPath TEXT NOT NULL,
        prdPath TEXT NOT NULL,
        cronExpression TEXT,
        triggerType TEXT NOT NULL DEFAULT 'manual',
        triggerConfig TEXT,
        roleTemplateName TEXT,
        budgetPreset TEXT,
        enabled INTEGER NOT NULL DEFAULT 1,
        lastRunAt TEXT,
        lastRunResult TEXT,
        nextRunAt TEXT,
        createdAt TEXT NOT NULL
    );
    CREATE INDEX idx_scheduled_run_project ON scheduled_run (projectPath);
    CREATE INDEX idx_scheduled_run_next ON scheduled_run (nextRunAt);
";

const V12_WORKSPACE: &str = "
    CREATE TABLE workspace (
        id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL,
        projectPath TEXT NOT NULL UNIQUE,
        color TEXT NOT NULL DEFAULT '#0DF170',
        icon TEXT,
        isActive INTEGER NOT NULL DEFAULT 0,
        maxSlots INTEGER NOT NULL DEFAULT 6,
        totalBudgetCents INTEGER,
        metadata TEXT,
        lastAccessedAt TEXT NOT NULL,
        createdAt TEXT NOT NULL
    );
    CREATE UNIQUE INDEX idx_workspace_path ON workspace (projectPath);
    CREATE INDEX idx_workspace_active ON workspace (isActive);
";

const V13_AGENT_RUNTIME: &str = "
    CREATE TABLE agent_runtime (
        id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL UNIQUE,
        runtimeType TEXT NOT NULL DEFAULT 'cli',
        command TEXT,
        url TEXT,
        dockerImage TEXT,
        healthCheckCommand TEXT,
        configSchema TEXT,
        configDefaults TEXT,
        capabilities TEXT NOT NULL DEFAULT '[]',
        icon TEXT,
        color TEXT,
        isBuiltin INTEGER NOT NULL DEFAULT 0,
        isEnabled INTEGER NOT NULL DEFAULT 1,
        createdAt TEXT NOT NULL
    );
    CREATE UNIQUE INDEX idx_runtime_name ON agent_runtime (name);
";

const V14_CONFIG_SNAPSHOT: &str = "
    CREATE TABLE config_snapshot (
        id TEXT PRIMARY KEY NOT NULL,
        sessionId TEXT REFERENCES cockpit_session(id) ON DELETE CASCADE,
        workspaceId TEXT REFERENCES workspace(id) ON DELETE CASCADE,
        configType TEXT NOT NULL,
        version INTEGER NOT NULL,
        data TEXT NOT NULL,
        diff TEXT,
        changedBy TEXT NOT NULL DEFAULT 'system',
        changeReason TEXT,
        createdAt TEXT NOT NULL
    );
    CREATE INDEX idx_config_snapshot_session ON config_snapshot (sessionId, configType, version);
    CREATE INDEX idx_config_snapshot_workspace ON config_snapshot (workspaceId);
";

const V15_LEARNING: &str = "
    CREATE TABLE learning_record (
        id TEXT PRIMARY KEY NOT NULL,
        sessionId TEXT NOT NULL REFERENCES cockpit_session(id) ON DELETE CASCADE,
        storyId TEXT NOT NULL,
        storyTitle TEXT NOT NULL,
        storyComplexity TEXT NOT NULL DEFAULT 'moderate',
        agentType TEXT NOT NULL,
        runtimeId TEXT REFERENCES agent_runtime(id),
        model TEXT,
        durationMs INTEGER NOT NULL DEFAULT 0,
        costCents INTEGER NOT NULL DEFAULT 0,
        filesChanged INTEGER NOT NULL DEFAULT 0,
        linesAdded INTEGER NOT NULL DEFAULT 0,
        linesRemoved INTEGER NOT NULL DEFAULT 0,
        testsRun INTEGER NOT NULL DEFAULT 0,
        testsPassed INTEGER NOT NULL DEFAULT 0,
        testsFailed INTEGER NOT NULL DEFAULT 0,
        conflictsEncountered INTEGER NOT NULL DEFAULT 0,
        retriesNeeded INTEGER NOT NULL DEFAULT 0,
        success INTEGER NOT NULL DEFAULT 1,
        failureReason TEXT,
        filePatterns TEXT NOT NULL DEFAULT '[]',
        createdAt TEXT NOT NULL
    );
    CREATE INDEX idx_learning_session ON learning_record (sessionId);
    CREATE INDEX idx_learning_agent ON learning_record (agentType);
    CREATE INDEX idx_learning_story ON learning_record (storyId);

    CREATE TABLE performance_profile (
        id TEXT PRIMARY KEY NOT NULL,
        agentType TEXT NOT NULL,
        runtimeId TEXT REFERENCES agent_runtime(id),
        taskCategory TEXT NOT NULL,
        totalExecutions INTEGER NOT NULL DEFAULT 0,
        successRate REAL NOT NULL DEFAULT 0.0,
        avgDurationMs INTEGER NOT NULL DEFAULT 0,
        avgCostCents INTEGER NOT NULL DEFAULT 0,
        avgFilesChanged REAL NOT NULL DEFAULT 0.0,
        avgTestPassRate REAL NOT NULL DEFAULT 0.0,
        conflictRate REAL NOT NULL DEFAULT 0.0,
        lastUpdatedAt TEXT NOT NULL
    );
    CREATE UNIQUE INDEX idx_perf_profile_unique ON performance_profile (agentType, taskCategory);
    CREATE INDEX idx_perf_profile_agent ON performance_profile (agentType);
";

const V16_AGENT_MEMORY: &str = "
    CREATE TABLE agent_memory (
        id TEXT PRIMARY KEY NOT NULL,
        agentType TEXT NOT NULL,
        domain TEXT NOT NULL,
        memoryType TEXT NOT NULL DEFAULT 'observation',
        content TEXT NOT NULL,
        confidence REAL NOT NULL DEFAULT 0.5,
        sourceSessionId TEXT,
        sourceStoryId TEXT,
        tags TEXT DEFAULT '[]',
        accessCount INTEGER NOT NULL DEFAULT 0,
        lastAccessedAt TEXT,
        createdAt TEXT NOT NULL
    );
    CREATE INDEX idx_agent_memory_agent ON agent_memory (agentType, domain);
    CREATE INDEX idx_agent_memory_type ON agent_memory (memoryType);
";

const V17_TRUST_SCORE: &str = "
    CREATE TABLE trust_score (
        id TEXT PRIMARY KEY NOT NULL,
        agentType TEXT NOT NULL,
        domain TEXT NOT NULL,
        score REAL NOT NULL DEFAULT 0.5,
        totalStories INTEGER NOT NULL DEFAULT 0,
        successfulStories INTEGER NOT NULL DEFAULT 0,
        totalTestsPassed INTEGER NOT NULL DEFAULT 0,
        totalTestsFailed INTEGER NOT NULL DEFAULT 0,
        autoMergeEnabled INTEGER NOT NULL DEFAULT 0,
        autoMergeThreshold REAL NOT NULL DEFAULT 0.9,
        lastComputedAt TEXT NOT NULL,
        UNIQUE(agentType, domain)
    );
    CREATE INDEX idx_trust_score_agent ON trust_score (agentType);
";

const V18_CLAUDE_SESSION_ID: &str = "
    ALTER TABLE agent_slot ADD COLUMN claudeSessionId TEXT;
";

const V19_CHAT_HISTORY: &str = "
    CREATE TABLE chat_history (
        id TEXT PRIMARY KEY NOT NULL,
        sessionId TEXT REFERENCES cockpit_session(id) ON DELETE CASCADE,
        role TEXT NOT NULL,
        content TEXT NOT NULL,
        mode TEXT,
        metadata TEXT,
        createdAt TEXT NOT NULL
    );
    CREATE INDEX idx_chat_history_session ON chat_history (sessionId);
    CREATE INDEX idx_chat_history_created ON chat_history (createdAt);

    CREATE TABLE cockpit_wake_prompt (
        id TEXT PRIMARY KEY NOT NULL,
        sessionId TEXT REFERENCES cockpit_session(id) ON DELETE CASCADE,
        prompt TEXT NOT NULL,
        observations TEXT,
        pendingActions TEXT,
        slotSummaries TEXT,
        createdAt TEXT NOT NULL
    );
    CREATE INDEX idx_wake_prompt_session ON cockpit_wake_prompt (sessionId);
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
        assert_eq!(count, 19);
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
            "org_role",
            "budget_config",
            "budget_alert",
            "heartbeat_config",
            "scheduled_run",
            "workspace",
            "agent_runtime",
            "config_snapshot",
            "learning_record",
            "performance_profile",
            "agent_memory",
            "trust_score",
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
        assert_eq!(count, 19);
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
