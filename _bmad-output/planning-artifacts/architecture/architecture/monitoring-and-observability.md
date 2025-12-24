# Monitoring and Observability

This document defines the comprehensive monitoring and observability strategy for Ferrocodex, ensuring proper visibility into application health, performance, and security across the desktop application's operations.

## Monitoring Philosophy

### Core Principles
- **Proactive Monitoring**: Detect issues before they impact users
- **Security-First Observability**: Monitor authentication, authorization, and security events
- **Performance Visibility**: Track application performance and resource usage
- **Privacy-Conscious**: Maintain user privacy while collecting necessary telemetry
- **Local-First**: Desktop application logs locally with optional cloud aggregation
- **Actionable Metrics**: Focus on metrics that drive operational decisions

### Monitoring Objectives
1. **Application Health**: Monitor system status, errors, and availability
2. **Security Monitoring**: Track authentication failures, suspicious activities, and access patterns
3. **Performance Tracking**: Monitor response times, resource usage, and bottlenecks
4. **User Experience**: Track feature usage and identify pain points
5. **Operational Intelligence**: Support troubleshooting and capacity planning

## Logging Strategy

### Backend Logging (Rust)

#### Logging Framework Configuration
```rust
// main.rs - Logging setup
use tracing::{info, warn, error, debug};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, fmt};
use std::path::PathBuf;

pub fn setup_logging(app_data_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let log_dir = app_data_dir.join("logs");
    std::fs::create_dir_all(&log_dir)?;
    
    let log_file = log_dir.join("ferrocodex.log");
    let file_appender = tracing_appender::rolling::daily(&log_dir, "ferrocodex.log");
    
    // Configure structured logging with multiple outputs
    let file_layer = fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false)
        .json(); // Structured JSON logs for parsing
    
    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .pretty(); // Human-readable console output
    
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info")))
        .with(file_layer)
        .with(console_layer)
        .init();
    
    info!("Logging initialized: {}", log_file.display());
    Ok(())
}
```

#### Structured Logging Standards
```rust
use tracing::{info, warn, error, debug, instrument, Span};
use serde_json::json;

// Structured logging with consistent fields
#[instrument(skip(db), fields(user_id = %user_id, operation = "user_creation"))]
pub async fn create_user(
    db: &Database,
    request: CreateUserRequest,
    user_id: i64,
) -> Result<UserInfo, String> {
    // Add dynamic fields to current span
    Span::current().record("username", &request.username);
    Span::current().record("role", &request.role.to_string());
    
    info!(
        user_id = user_id,
        username = %request.username,
        role = %request.role,
        "User creation started"
    );
    
    match validate_user_request(&request) {
        Ok(_) => {
            debug!("User validation passed");
        }
        Err(e) => {
            warn!(
                error = %e,
                validation_failures = ?extract_validation_errors(&e),
                "User validation failed"
            );
            return Err(e);
        }
    }
    
    // ... implementation
    
    match result {
        Ok(user) => {
            info!(
                created_user_id = user.id,
                username = %user.username,
                role = %user.role,
                operation_duration_ms = start_time.elapsed().as_millis(),
                "User created successfully"
            );
            Ok(user)
        }
        Err(e) => {
            error!(
                error = %e,
                error_type = "user_creation_failed",
                operation_duration_ms = start_time.elapsed().as_millis(),
                "User creation failed"
            );
            Err(e)
        }
    }
}
```

#### Security Event Logging
```rust
// Security-specific logging module
pub mod security_logging {
    use tracing::{info, warn, error};
    use serde_json::json;
    use std::net::IpAddr;
    
    pub fn log_authentication_success(username: &str, session_id: &str) {
        info!(
            event_type = "auth_success",
            username = username,
            session_id = session_id,
            timestamp = chrono::Utc::now().to_rfc3339(),
            "User authentication successful"
        );
    }
    
    pub fn log_authentication_failure(username: &str, failure_reason: &str, source_ip: Option<IpAddr>) {
        warn!(
            event_type = "auth_failure",
            username = username,
            failure_reason = failure_reason,
            source_ip = ?source_ip,
            timestamp = chrono::Utc::now().to_rfc3339(),
            "User authentication failed"
        );
    }
    
    pub fn log_authorization_failure(username: &str, requested_resource: &str, required_role: &str) {
        warn!(
            event_type = "authz_failure",
            username = username,
            requested_resource = requested_resource,
            required_role = required_role,
            current_role = "unknown", // Set from context
            timestamp = chrono::Utc::now().to_rfc3339(),
            "Authorization failed"
        );
    }
    
    pub fn log_rate_limit_exceeded(username: &str, operation: &str, current_count: u32, limit: u32) {
        warn!(
            event_type = "rate_limit_exceeded",
            username = username,
            operation = operation,
            current_count = current_count,
            limit = limit,
            timestamp = chrono::Utc::now().to_rfc3339(),
            "Rate limit exceeded"
        );
    }
    
    pub fn log_suspicious_activity(username: &str, activity_type: &str, details: serde_json::Value) {
        error!(
            event_type = "suspicious_activity",
            username = username,
            activity_type = activity_type,
            details = ?details,
            timestamp = chrono::Utc::now().to_rfc3339(),
            "Suspicious activity detected"
        );
    }
}
```

#### Performance Metrics Logging
```rust
// Performance monitoring utilities
pub mod performance_logging {
    use tracing::{info, warn};
    use std::time::Instant;
    
    pub struct OperationTimer {
        operation: String,
        start_time: Instant,
        threshold_ms: u128,
    }
    
    impl OperationTimer {
        pub fn new(operation: String, threshold_ms: u128) -> Self {
            Self {
                operation,
                start_time: Instant::now(),
                threshold_ms,
            }
        }
        
        pub fn finish(self) {
            let duration_ms = self.start_time.elapsed().as_millis();
            
            if duration_ms > self.threshold_ms {
                warn!(
                    event_type = "slow_operation",
                    operation = %self.operation,
                    duration_ms = duration_ms,
                    threshold_ms = self.threshold_ms,
                    "Operation exceeded performance threshold"
                );
            } else {
                info!(
                    event_type = "operation_completed",
                    operation = %self.operation,
                    duration_ms = duration_ms,
                    "Operation completed within threshold"
                );
            }
        }
    }
    
    // Database query performance monitoring
    pub async fn monitored_db_query<T, F, Fut>(
        query_name: &str,
        operation: F,
    ) -> Result<T, String>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, String>>,
    {
        let timer = OperationTimer::new(format!("db_query_{}", query_name), 500); // 500ms threshold
        let result = operation().await;
        timer.finish();
        
        match &result {
            Ok(_) => {
                info!(
                    event_type = "db_query_success",
                    query_name = query_name,
                    "Database query completed successfully"
                );
            }
            Err(e) => {
                warn!(
                    event_type = "db_query_error",
                    query_name = query_name,
                    error = %e,
                    "Database query failed"
                );
            }
        }
        
        result
    }
}
```

### Frontend Logging (TypeScript)

#### Client-Side Logging Service
```typescript
// Centralized logging service for frontend
interface LogEntry {
  timestamp: string;
  level: 'debug' | 'info' | 'warn' | 'error';
  message: string;
  context?: string;
  component?: string;
  userId?: string;
  sessionId?: string;
  details?: Record<string, unknown>;
  error?: {
    name: string;
    message: string;
    stack?: string;
  };
}

class FrontendLogger {
  private logBuffer: LogEntry[] = [];
  private maxBufferSize = 100;
  private flushInterval = 30000; // 30 seconds
  private userId: string | null = null;
  private sessionId: string | null = null;
  
  constructor() {
    // Auto-flush logs periodically
    setInterval(() => {
      this.flush();
    }, this.flushInterval);
    
    // Flush logs before page unload
    window.addEventListener('beforeunload', () => {
      this.flush();
    });
  }
  
  setUser(userId: string, sessionId: string) {
    this.userId = userId;
    this.sessionId = sessionId;
  }
  
  clearUser() {
    this.userId = null;
    this.sessionId = null;
  }
  
  debug(message: string, context?: string, details?: Record<string, unknown>) {
    this.log('debug', message, context, details);
  }
  
  info(message: string, context?: string, details?: Record<string, unknown>) {
    this.log('info', message, context, details);
  }
  
  warn(message: string, context?: string, details?: Record<string, unknown>) {
    this.log('warn', message, context, details);
  }
  
  error(message: string, error?: Error, context?: string, details?: Record<string, unknown>) {
    const errorDetails = error ? {
      name: error.name,
      message: error.message,
      stack: error.stack,
    } : undefined;
    
    this.log('error', message, context, { ...details, error: errorDetails });
  }
  
  private log(
    level: LogEntry['level'],
    message: string,
    context?: string,
    details?: Record<string, unknown>
  ) {
    const entry: LogEntry = {
      timestamp: new Date().toISOString(),
      level,
      message,
      context,
      userId: this.userId || undefined,
      sessionId: this.sessionId || undefined,
      details,
      component: this.getCurrentComponent(),
    };
    
    // Console output for development
    if (process.env.NODE_ENV === 'development') {
      const consoleMethod = level === 'debug' ? 'debug' :
                           level === 'info' ? 'info' :
                           level === 'warn' ? 'warn' : 'error';
      console[consoleMethod](`[${level.toUpperCase()}] ${message}`, entry);
    }
    
    // Add to buffer
    this.logBuffer.push(entry);
    
    // Flush if buffer is full
    if (this.logBuffer.length >= this.maxBufferSize) {
      this.flush();
    }
  }
  
  private getCurrentComponent(): string | undefined {
    // Extract component name from current React component
    const stack = new Error().stack;
    if (stack) {
      const match = stack.match(/at (\w+Component|\w+\.tsx)/);
      return match ? match[1] : undefined;
    }
    return undefined;
  }
  
  private async flush() {
    if (this.logBuffer.length === 0) return;
    
    const logsToSend = [...this.logBuffer];
    this.logBuffer = [];
    
    try {
      // Send logs to backend for aggregation
      await invoke('log_frontend_events', { logs: logsToSend });
    } catch (error) {
      console.error('Failed to send logs to backend:', error);
      // Re-add failed logs to buffer (with limit to prevent infinite growth)
      this.logBuffer = [...logsToSend.slice(-50), ...this.logBuffer];
    }
  }
  
  // Performance timing utilities
  startTiming(operation: string): () => void {
    const startTime = performance.now();
    
    return () => {
      const duration = performance.now() - startTime;
      this.info(`Operation completed: ${operation}`, 'performance', {
        operation,
        duration_ms: duration,
      });
      
      if (duration > 1000) { // Warn for operations over 1 second
        this.warn(`Slow operation detected: ${operation}`, 'performance', {
          operation,
          duration_ms: duration,
        });
      }
    };
  }
  
  // User interaction tracking
  trackUserAction(action: string, component: string, details?: Record<string, unknown>) {
    this.info(`User action: ${action}`, 'user_interaction', {
      action,
      component,
      ...details,
    });
  }
  
  // Error tracking with context
  trackError(error: Error, context?: string, additionalInfo?: Record<string, unknown>) {
    this.error(
      `Unhandled error: ${error.message}`,
      error,
      context || 'error_boundary',
      {
        errorName: error.name,
        ...additionalInfo,
      }
    );
  }
}

export const logger = new FrontendLogger();
```

#### Component-Level Logging
```typescript
// Hook for component-level logging
export function useLogger(componentName: string) {
  const componentLogger = useMemo(() => {
    return {
      debug: (message: string, details?: Record<string, unknown>) =>
        logger.debug(message, componentName, details),
      info: (message: string, details?: Record<string, unknown>) =>
        logger.info(message, componentName, details),
      warn: (message: string, details?: Record<string, unknown>) =>
        logger.warn(message, componentName, details),
      error: (message: string, error?: Error, details?: Record<string, unknown>) =>
        logger.error(message, error, componentName, details),
      trackAction: (action: string, details?: Record<string, unknown>) =>
        logger.trackUserAction(action, componentName, details),
      startTiming: (operation: string) =>
        logger.startTiming(`${componentName}.${operation}`),
    };
  }, [componentName]);
  
  return componentLogger;
}

// Example component usage
const UserManagement: React.FC = () => {
  const log = useLogger('UserManagement');
  const [users, setUsers] = useState<UserInfo[]>([]);
  
  const loadUsers = useCallback(async () => {
    const endTiming = log.startTiming('loadUsers');
    
    try {
      log.info('Loading users');
      const result = await invoke<UserInfo[]>('get_users');
      setUsers(result);
      log.info('Users loaded successfully', { count: result.length });
    } catch (error) {
      log.error('Failed to load users', error as Error);
      throw error;
    } finally {
      endTiming();
    }
  }, [log]);
  
  const handleUserCreate = useCallback(async (userData: CreateUserRequest) => {
    log.trackAction('create_user_clicked', { username: userData.username });
    
    try {
      const newUser = await invoke<UserInfo>('create_user', { userData });
      log.info('User created successfully', { userId: newUser.id });
      setUsers(prev => [...prev, newUser]);
    } catch (error) {
      log.error('User creation failed', error as Error, { username: userData.username });
      throw error;
    }
  }, [log]);
  
  // ... component implementation
};
```

## Metrics Collection

### Application Metrics
```rust
// Metrics collection using a simple in-memory counter system
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct ApplicationMetrics {
    counters: Arc<Mutex<HashMap<String, u64>>>,
    timers: Arc<Mutex<HashMap<String, Vec<u128>>>>,
    start_time: Instant,
}

impl ApplicationMetrics {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(Mutex::new(HashMap::new())),
            timers: Arc::new(Mutex::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }
    
    pub fn increment_counter(&self, name: &str) {
        let mut counters = self.counters.lock().unwrap();
        *counters.entry(name.to_string()).or_insert(0) += 1;
    }
    
    pub fn record_timer(&self, name: &str, duration_ms: u128) {
        let mut timers = self.timers.lock().unwrap();
        timers.entry(name.to_string()).or_insert_with(Vec::new).push(duration_ms);
    }
    
    pub fn get_metrics_summary(&self) -> MetricsSummary {
        let counters = self.counters.lock().unwrap().clone();
        let timers = self.timers.lock().unwrap();
        
        let timer_stats: HashMap<String, TimerStats> = timers
            .iter()
            .map(|(name, values)| {
                let count = values.len();
                let sum: u128 = values.iter().sum();
                let avg = if count > 0 { sum / count as u128 } else { 0 };
                let min = values.iter().min().copied().unwrap_or(0);
                let max = values.iter().max().copied().unwrap_or(0);
                
                (name.clone(), TimerStats { count, avg, min, max })
            })
            .collect();
        
        MetricsSummary {
            uptime_seconds: self.start_time.elapsed().as_secs(),
            counters,
            timers: timer_stats,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MetricsSummary {
    pub uptime_seconds: u64,
    pub counters: HashMap<String, u64>,
    pub timers: HashMap<String, TimerStats>,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct TimerStats {
    pub count: usize,
    pub avg: u128,
    pub min: u128,
    pub max: u128,
}
```

### Key Metrics to Track

#### Security Metrics
```rust
// Security-related metrics collection
impl ApplicationMetrics {
    pub fn track_security_events(&self) {
        // Authentication metrics
        self.increment_counter("auth.login_attempts");
        self.increment_counter("auth.login_success");
        self.increment_counter("auth.login_failures");
        self.increment_counter("auth.password_changes");
        self.increment_counter("auth.session_timeouts");
        
        // Authorization metrics
        self.increment_counter("authz.access_granted");
        self.increment_counter("authz.access_denied");
        self.increment_counter("authz.role_violations");
        
        // Rate limiting metrics
        self.increment_counter("rate_limit.triggers");
        self.increment_counter("rate_limit.user_blocked");
        
        // Data access metrics
        self.increment_counter("data.user_created");
        self.increment_counter("data.user_modified");
        self.increment_counter("data.user_deleted");
        self.increment_counter("data.config_uploaded");
        self.increment_counter("data.config_downloaded");
        self.increment_counter("data.firmware_uploaded");
    }
}
```

#### Performance Metrics
```rust
impl ApplicationMetrics {
    pub fn track_performance(&self, operation: &str, duration_ms: u128) {
        self.record_timer(&format!("perf.{}", operation), duration_ms);
        
        // Track specific performance thresholds
        if duration_ms > 1000 {
            self.increment_counter("perf.slow_operations");
        }
        if duration_ms > 5000 {
            self.increment_counter("perf.very_slow_operations");
        }
        
        // Database performance
        if operation.starts_with("db_") {
            self.record_timer("perf.database_operations", duration_ms);
            
            if duration_ms > 500 {
                self.increment_counter("perf.slow_db_queries");
            }
        }
        
        // File operations
        if operation.starts_with("file_") {
            self.record_timer("perf.file_operations", duration_ms);
        }
    }
}
```

#### Business Metrics
```typescript
// Frontend business metrics tracking
interface BusinessMetrics {
  // Feature usage
  dashboard_views: number;
  user_management_accessed: number;
  asset_management_accessed: number;
  configuration_uploads: number;
  configuration_downloads: number;
  firmware_uploads: number;
  firmware_analysis_runs: number;
  
  // User behavior
  session_duration_minutes: number;
  actions_per_session: number;
  error_rate_percent: number;
  feature_adoption_rate: number;
}

class BusinessMetricsCollector {
  private metrics: Partial<BusinessMetrics> = {};
  private sessionStart = Date.now();
  
  trackFeatureUsage(feature: keyof BusinessMetrics) {
    this.metrics[feature] = (this.metrics[feature] || 0) + 1;
    logger.info(`Feature used: ${feature}`, 'metrics', { 
      count: this.metrics[feature] 
    });
  }
  
  getSessionMetrics(): Partial<BusinessMetrics> & { session_duration_minutes: number } {
    const sessionDuration = (Date.now() - this.sessionStart) / 1000 / 60;
    
    return {
      ...this.metrics,
      session_duration_minutes: sessionDuration,
    };
  }
  
  async reportMetrics() {
    const sessionMetrics = this.getSessionMetrics();
    
    try {
      await invoke('report_business_metrics', { metrics: sessionMetrics });
      logger.info('Business metrics reported', 'metrics', sessionMetrics);
    } catch (error) {
      logger.error('Failed to report business metrics', error as Error, 'metrics');
    }
  }
}

export const businessMetrics = new BusinessMetricsCollector();
```

## Health Monitoring

### Application Health Checks
```rust
// Health check system
#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub overall_status: ServiceStatus,
    pub components: HashMap<String, ComponentHealth>,
    pub timestamp: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Serialize)]
pub struct ComponentHealth {
    pub status: ServiceStatus,
    pub last_check: String,
    pub error_message: Option<String>,
    pub response_time_ms: Option<u128>,
}

pub struct HealthChecker {
    database: Arc<Database>,
    metrics: Arc<ApplicationMetrics>,
    start_time: Instant,
}

impl HealthChecker {
    pub async fn check_health(&self) -> HealthStatus {
        let mut components = HashMap::new();
        
        // Database health
        let db_health = self.check_database_health().await;
        components.insert("database".to_string(), db_health);
        
        // File system health
        let fs_health = self.check_filesystem_health().await;
        components.insert("filesystem".to_string(), fs_health);
        
        // Memory health
        let memory_health = self.check_memory_health();
        components.insert("memory".to_string(), memory_health);
        
        // Determine overall status
        let overall_status = if components.values().all(|c| matches!(c.status, ServiceStatus::Healthy)) {
            ServiceStatus::Healthy
        } else if components.values().any(|c| matches!(c.status, ServiceStatus::Unhealthy)) {
            ServiceStatus::Unhealthy
        } else {
            ServiceStatus::Degraded
        };
        
        HealthStatus {
            overall_status,
            components,
            timestamp: chrono::Utc::now().to_rfc3339(),
            uptime_seconds: self.start_time.elapsed().as_secs(),
        }
    }
    
    async fn check_database_health(&self) -> ComponentHealth {
        let start = Instant::now();
        
        match self.database.execute_simple_query("SELECT 1").await {
            Ok(_) => ComponentHealth {
                status: ServiceStatus::Healthy,
                last_check: chrono::Utc::now().to_rfc3339(),
                error_message: None,
                response_time_ms: Some(start.elapsed().as_millis()),
            },
            Err(e) => ComponentHealth {
                status: ServiceStatus::Unhealthy,
                last_check: chrono::Utc::now().to_rfc3339(),
                error_message: Some(e.to_string()),
                response_time_ms: Some(start.elapsed().as_millis()),
            },
        }
    }
    
    async fn check_filesystem_health(&self) -> ComponentHealth {
        let start = Instant::now();
        
        // Check if we can write to the data directory
        let temp_file = std::env::temp_dir().join("ferrocodex_health_check.tmp");
        
        match std::fs::write(&temp_file, "health_check") {
            Ok(_) => {
                std::fs::remove_file(&temp_file).ok();
                ComponentHealth {
                    status: ServiceStatus::Healthy,
                    last_check: chrono::Utc::now().to_rfc3339(),
                    error_message: None,
                    response_time_ms: Some(start.elapsed().as_millis()),
                }
            }
            Err(e) => ComponentHealth {
                status: ServiceStatus::Unhealthy,
                last_check: chrono::Utc::now().to_rfc3339(),
                error_message: Some(e.to_string()),
                response_time_ms: Some(start.elapsed().as_millis()),
            },
        }
    }
    
    fn check_memory_health(&self) -> ComponentHealth {
        // Simple memory check - in a real application, you might use more sophisticated metrics
        let available_memory = sys_info::mem_info().map(|info| info.avail).unwrap_or(0);
        let threshold_mb = 100; // Minimum 100MB required
        
        if available_memory > threshold_mb * 1024 {
            ComponentHealth {
                status: ServiceStatus::Healthy,
                last_check: chrono::Utc::now().to_rfc3339(),
                error_message: None,
                response_time_ms: None,
            }
        } else {
            ComponentHealth {
                status: ServiceStatus::Degraded,
                last_check: chrono::Utc::now().to_rfc3339(),
                error_message: Some(format!("Low memory: {}KB available", available_memory)),
                response_time_ms: None,
            }
        }
    }
}

// Tauri command to expose health status
#[tauri::command]
pub async fn get_health_status(
    health_checker: State<'_, Arc<HealthChecker>>,
) -> Result<HealthStatus, String> {
    Ok(health_checker.check_health().await)
}
```

## Log Management

### Log Rotation and Retention
```rust
// Log management configuration
pub struct LogManager {
    log_dir: PathBuf,
    max_file_size_mb: u64,
    retention_days: u32,
}

impl LogManager {
    pub fn new(log_dir: PathBuf) -> Self {
        Self {
            log_dir,
            max_file_size_mb: 100, // 100MB per file
            retention_days: 30,    // Keep logs for 30 days
        }
    }
    
    pub async fn cleanup_old_logs(&self) -> Result<(), Box<dyn std::error::Error>> {
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(self.retention_days as i64);
        
        let mut entries = tokio::fs::read_dir(&self.log_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("log") {
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(modified) = metadata.modified() {
                        let modified_date = chrono::DateTime::<chrono::Utc>::from(modified);
                        
                        if modified_date < cutoff_date {
                            tokio::fs::remove_file(&path).await?;
                            info!("Removed old log file: {}", path.display());
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    pub async fn get_log_files_info(&self) -> Result<Vec<LogFileInfo>, Box<dyn std::error::Error>> {
        let mut log_files = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.log_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("log") {
                if let Ok(metadata) = entry.metadata().await {
                    log_files.push(LogFileInfo {
                        name: path.file_name().unwrap().to_string_lossy().to_string(),
                        size_bytes: metadata.len(),
                        modified: metadata.modified().ok()
                            .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()),
                    });
                }
            }
        }
        
        log_files.sort_by(|a, b| b.modified.cmp(&a.modified));
        Ok(log_files)
    }
}

#[derive(Debug, Serialize)]
pub struct LogFileInfo {
    pub name: String,
    pub size_bytes: u64,
    pub modified: Option<String>,
}
```

### Log Analysis and Alerting
```rust
// Basic log analysis for desktop application
pub struct LogAnalyzer {
    error_threshold: u32,
    warning_threshold: u32,
    time_window_minutes: u32,
}

impl LogAnalyzer {
    pub async fn analyze_recent_logs(&self) -> LogAnalysis {
        let since = chrono::Utc::now() - chrono::Duration::minutes(self.time_window_minutes as i64);
        
        // In a real implementation, you would parse log files
        // This is a simplified version
        let error_count = self.count_log_entries_since("ERROR", since).await;
        let warning_count = self.count_log_entries_since("WARN", since).await;
        
        let mut alerts = Vec::new();
        
        if error_count >= self.error_threshold {
            alerts.push(LogAlert {
                severity: AlertSeverity::High,
                message: format!("High error rate: {} errors in {} minutes", error_count, self.time_window_minutes),
                metric: "error_rate".to_string(),
                value: error_count,
                threshold: self.error_threshold,
            });
        }
        
        if warning_count >= self.warning_threshold {
            alerts.push(LogAlert {
                severity: AlertSeverity::Medium,
                message: format!("High warning rate: {} warnings in {} minutes", warning_count, self.time_window_minutes),
                metric: "warning_rate".to_string(),
                value: warning_count,
                threshold: self.warning_threshold,
            });
        }
        
        LogAnalysis {
            time_window_start: since.to_rfc3339(),
            time_window_end: chrono::Utc::now().to_rfc3339(),
            error_count,
            warning_count,
            alerts,
        }
    }
    
    async fn count_log_entries_since(&self, level: &str, since: chrono::DateTime<chrono::Utc>) -> u32 {
        // Simplified implementation - in practice, you'd parse actual log files
        // or maintain counters in memory
        0
    }
}

#[derive(Debug, Serialize)]
pub struct LogAnalysis {
    pub time_window_start: String,
    pub time_window_end: String,
    pub error_count: u32,
    pub warning_count: u32,
    pub alerts: Vec<LogAlert>,
}

#[derive(Debug, Serialize)]
pub struct LogAlert {
    pub severity: AlertSeverity,
    pub message: String,
    pub metric: String,
    pub value: u32,
    pub threshold: u32,
}

#[derive(Debug, Serialize)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}
```

## Privacy and Compliance

### Data Collection Policy
```rust
// Privacy-conscious data collection
pub struct PrivacyPolicy {
    pub collect_usage_metrics: bool,
    pub collect_performance_metrics: bool,
    pub collect_error_reports: bool,
    pub include_user_identifiers: bool,
    pub data_retention_days: u32,
}

impl Default for PrivacyPolicy {
    fn default() -> Self {
        Self {
            collect_usage_metrics: true,     // Basic feature usage
            collect_performance_metrics: true, // Performance optimization
            collect_error_reports: true,     // Bug fixing
            include_user_identifiers: false, // Privacy-first
            data_retention_days: 7,          // Short retention
        }
    }
}

pub fn sanitize_log_entry(entry: &mut LogEntry, policy: &PrivacyPolicy) {
    if !policy.include_user_identifiers {
        // Hash or remove personally identifiable information
        if let Some(username) = entry.username.as_mut() {
            *username = hash_identifier(username);
        }
        
        entry.ip_address = None;
        entry.email = None;
    }
    
    // Remove sensitive fields from error details
    if let Some(details) = entry.details.as_mut() {
        details.remove("password");
        details.remove("token");
        details.remove("session_id");
        details.remove("api_key");
    }
}

fn hash_identifier(identifier: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(identifier.as_bytes());
    format!("{:x}", hasher.finalize())[..8].to_string() // First 8 chars of hash
}
```

## Monitoring Dashboard (Frontend)

### Real-time Monitoring Component
```typescript
// Real-time monitoring dashboard component
interface MonitoringData {
  healthStatus: HealthStatus;
  metrics: MetricsSummary;
  recentLogs: LogEntry[];
  alerts: LogAlert[];
}

const MonitoringDashboard: React.FC = () => {
  const [monitoringData, setMonitoringData] = useState<MonitoringData | null>(null);
  const [loading, setLoading] = useState(true);
  const log = useLogger('MonitoringDashboard');
  
  const loadMonitoringData = useCallback(async () => {
    try {
      const [healthStatus, metrics, recentLogs, alerts] = await Promise.all([
        invoke<HealthStatus>('get_health_status'),
        invoke<MetricsSummary>('get_metrics_summary'),
        invoke<LogEntry[]>('get_recent_logs', { limit: 50 }),
        invoke<LogAlert[]>('get_active_alerts'),
      ]);
      
      setMonitoringData({ healthStatus, metrics, recentLogs, alerts });
      log.info('Monitoring data loaded successfully');
    } catch (error) {
      log.error('Failed to load monitoring data', error as Error);
      throw error;
    } finally {
      setLoading(false);
    }
  }, [log]);
  
  // Auto-refresh every 30 seconds
  useEffect(() => {
    loadMonitoringData();
    const interval = setInterval(loadMonitoringData, 30000);
    return () => clearInterval(interval);
  }, [loadMonitoringData]);
  
  if (loading || !monitoringData) {
    return <Spin size="large" />;
  }
  
  const { healthStatus, metrics, recentLogs, alerts } = monitoringData;
  
  return (
    <div className="monitoring-dashboard">
      <Row gutter={[16, 16]}>
        {/* Health Status */}
        <Col span={24}>
          <Card title="System Health">
            <div className="health-overview">
              <Tag
                color={
                  healthStatus.overall_status === 'Healthy' ? 'green' :
                  healthStatus.overall_status === 'Degraded' ? 'orange' : 'red'
                }
              >
                {healthStatus.overall_status}
              </Tag>
              <Text>Uptime: {Math.floor(healthStatus.uptime_seconds / 3600)}h {Math.floor((healthStatus.uptime_seconds % 3600) / 60)}m</Text>
            </div>
            
            <Row gutter={16} style={{ marginTop: 16 }}>
              {Object.entries(healthStatus.components).map(([name, component]) => (
                <Col span={8} key={name}>
                  <Card size="small" title={name}>
                    <Tag
                      color={
                        component.status === 'Healthy' ? 'green' :
                        component.status === 'Degraded' ? 'orange' : 'red'
                      }
                    >
                      {component.status}
                    </Tag>
                    {component.response_time_ms && (
                      <div>Response: {component.response_time_ms}ms</div>
                    )}
                    {component.error_message && (
                      <div style={{ color: 'red', fontSize: '12px' }}>
                        {component.error_message}
                      </div>
                    )}
                  </Card>
                </Col>
              ))}
            </Row>
          </Card>
        </Col>
        
        {/* Alerts */}
        {alerts.length > 0 && (
          <Col span={24}>
            <Card title="Active Alerts">
              {alerts.map((alert, index) => (
                <Alert
                  key={index}
                  type={
                    alert.severity === 'Critical' || alert.severity === 'High' ? 'error' :
                    alert.severity === 'Medium' ? 'warning' : 'info'
                  }
                  message={alert.message}
                  description={`${alert.metric}: ${alert.value} (threshold: ${alert.threshold})`}
                  style={{ marginBottom: 8 }}
                />
              ))}
            </Card>
          </Col>
        )}
        
        {/* Metrics Summary */}
        <Col span={12}>
          <Card title="Performance Metrics">
            <Table
              size="small"
              dataSource={Object.entries(metrics.timers).map(([name, stats]) => ({
                key: name,
                operation: name,
                ...stats,
              }))}
              columns={[
                { title: 'Operation', dataIndex: 'operation', key: 'operation' },
                { title: 'Count', dataIndex: 'count', key: 'count' },
                { title: 'Avg (ms)', dataIndex: 'avg', key: 'avg' },
                { title: 'Min (ms)', dataIndex: 'min', key: 'min' },
                { title: 'Max (ms)', dataIndex: 'max', key: 'max' },
              ]}
              pagination={false}
            />
          </Card>
        </Col>
        
        {/* Event Counters */}
        <Col span={12}>
          <Card title="Event Counters">
            <Table
              size="small"
              dataSource={Object.entries(metrics.counters).map(([name, count]) => ({
                key: name,
                event: name,
                count,
              }))}
              columns={[
                { title: 'Event', dataIndex: 'event', key: 'event' },
                { title: 'Count', dataIndex: 'count', key: 'count' },
              ]}
              pagination={false}
            />
          </Card>
        </Col>
        
        {/* Recent Logs */}
        <Col span={24}>
          <Card title="Recent Logs">
            <Table
              size="small"
              dataSource={recentLogs}
              columns={[
                { title: 'Time', dataIndex: 'timestamp', key: 'timestamp', width: 180 },
                { 
                  title: 'Level', 
                  dataIndex: 'level', 
                  key: 'level', 
                  width: 80,
                  render: (level: string) => (
                    <Tag color={
                      level === 'error' ? 'red' :
                      level === 'warn' ? 'orange' :
                      level === 'info' ? 'blue' : 'default'
                    }>
                      {level.toUpperCase()}
                    </Tag>
                  )
                },
                { title: 'Component', dataIndex: 'component', key: 'component', width: 120 },
                { title: 'Message', dataIndex: 'message', key: 'message' },
              ]}
              scroll={{ y: 400 }}
              pagination={{ pageSize: 20 }}
            />
          </Card>
        </Col>
      </Row>
    </div>
  );
};
```

This comprehensive monitoring and observability strategy ensures complete visibility into the Ferrocodex application's health, performance, and security while respecting user privacy and providing actionable insights for maintenance and optimization.