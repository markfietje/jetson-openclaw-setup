# Brain Server Multi-Agent Implementation Plan

**Goal:** Make brain-server 100% correct, secure, efficient, and production-ready  
**Execution Model:** Sequential agents (one completes before next starts)  
**Context Preservation:** Each agent reads prior agent's output and AGENTS.md updates

---

## Phase 1: Security & Code Quality (Agent 1-2)

### Agent 1: Fix Critical Security Issues
**Priority:** CRITICAL  
**Estimated Work:** 2-3 hours

#### Tasks:
1. **Fix CORS Configuration** (`main.rs:1405-1408`)
   ```rust
   // Before (INSECURE):
   let cors = CorsLayer::new()
       .allow_origin(Any)
       .allow_methods(Any)
       .allow_headers(Any);
   
   // After (SECURE):
   let allowed_origins: Vec<Origin> = std::env::var("CORS_ORIGINS")
       .unwrap_or_else(|_| "http://localhost:3000".to_string())
       .split(',')
       .map(|s| s.trim().parse::<Origin>().unwrap_or(Origin::any()))
       .collect();
   
   let cors = CorsLayer::new()
       .allow_origin(allowed_origins)
       .allow_methods(GET | POST | PUT | DELETE)
       .allow_headers(Any);
   ```

2. **Add Environment-Based Configuration**
   - Add `CORS_ORIGINS` env var support
   - Add `RATE_LIMIT_REQUESTS` env var
   - Add `RATE_LIMIT_WINDOW_SECS` env var

3. **Verify no other security issues**
   - Check input validation on all endpoints
   - Ensure no SQL injection vectors
   - Review authentication/authorization (if any)

#### Deliverables:
- Updated `main.rs` with secure CORS
- Environment variable documentation in README

---

### Agent 2: Remove Dead Code & Refactor
**Priority:** HIGH  
**Estimated Work:** 2-3 hours

#### Tasks:
1. **Remove unused RateLimiter** (`main.rs:100-155`)
   - Option A: Actually implement rate limiting middleware
   - Option B: Remove entirely if not needed
   - **Decision:** Remove to keep code minimal (can add later if needed)

2. **Clean up EntityExtractor** (`extractor.rs:10-11`)
   - Convert from zero-sized struct to module-level functions
   - Simplify the module structure

3. **Remove unused RegexPatterns::compile_all()** (`domains.rs:182-210`)
   - Either implement regex-based extraction
   - Or remove dead code with `#[allow(dead_code)]`

4. **Review and remove any other dead code**
   - Run `cargo clippy -- -D dead_code`
   - Fix all warnings

#### Deliverables:
- Clean codebase with zero dead code warnings
- Refactored annotator module

---

## Phase 2: Performance & Efficiency (Agent 3-4)

### Agent 3: Optimize Search & Database
**Priority:** HIGH  
**Estimated Work:** 4-5 hours

#### Tasks:
1. **Fix Inefficient Search Pagination** (`main.rs:555-596`)
   - Replace OFFSET-based pagination with cursor-based
   - Add proper indexed columns for sorting
   - Implement batch processing with seek optimization

2. **Add Missing Database Indexes** (`main.rs:278-377`)
   ```sql
   -- Add to migration:
   CREATE INDEX IF NOT EXISTS idx_knowledge_created_at ON knowledge(created_at DESC);
   CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);
   CREATE INDEX IF NOT EXISTS idx_relationships_type ON relationships(relation_type);
   ```

3. **Optimize Connection Pool Usage**
   - Review pool size for production workload
   - Add connection pool metrics to health endpoint

4. **Reduce Memory Allocations**
   - Use `&str` instead of `String` where possible
   - Pre-allocate vectors with capacity
   - Use streaming for large responses

#### Deliverables:
- Optimized search with cursor-based pagination
- New database indexes
- Performance improvements documented

---

### Agent 4: Configuration & Constants
**Priority:** MEDIUM  
**Estimated Work:** 2-3 hours

#### Tasks:
1. **Extract Magic Numbers to Constants** (`main.rs`)
   ```rust
   // Create config module:
   pub mod config {
       pub const MODEL_ID: &str = "minishlab/potion-retrieval-32M";
       pub const DEFAULT_K: usize = 5;
       pub const MAX_K: usize = 100;
       pub const SERVER_VERSION: &str = "0.8.1"; // Bump version
       
       // Timeouts
       pub const REQUEST_TIMEOUT_SECS: u64 = 30;
       pub const SEARCH_TIMEOUT_SECS: u64 = 8;
       pub const HEALTH_TIMEOUT_SECS: u64 = 3;
       pub const SHUTDOWN_DRAIN_SECS: u64 = 60;
       
       // Limits
       pub const MAX_REQUEST_SIZE: usize = 1024 * 1024;
       pub const MAX_QUERY_LENGTH: usize = 2000;
       pub const SEARCH_BATCH_SIZE: usize = 500;
       
       // Pool
       pub const POOL_MAX_SIZE: u32 = 20;
       pub const POOL_MIN_IDLE: u32 = 2;
   }
   ```

2. **Improve Content Filtering** (`main.rs:1048-1060`)
   - Make suspicious patterns configurable
   - Remove overly broad patterns (`"def "`, `"import "`)
   - Add to config module

3. **Add Configuration Validation**
   - Validate all env vars at startup
   - Provide clear error messages

#### Deliverables:
- Centralized configuration module
- Configurable content filtering
- Better error messages

---

## Phase 3: Testing & Reliability (Agent 5-6)

### Agent 5: Comprehensive Testing
**Priority:** HIGH  
**Estimated Work:** 4-5 hours

#### Tasks:
1. **Add Integration Tests**
   - Test all endpoints with real database
   - Test authentication (if added)
   - Test rate limiting (if implemented)

2. **Add Performance Benchmarks**
   - Use `criterion` crate for benchmarks
   - Measure search latency
   - Measure embedding generation time

3. **Add Chaos Testing**
   - Simulate database failures
   - Simulate slow responses
   - Test graceful degradation

4. **Improve Unit Test Coverage**
   - Target 80%+ coverage
   - Add edge case tests

#### Deliverables:
- Integration test suite
- Performance benchmarks
- Test documentation

---

### Agent 6: Error Handling & Logging
**Priority:** MEDIUM  
**Estimated Work:** 2-3 hours

#### Tasks:
1. **Standardize Error Handling**
   - Create unified error type
   - Add error context propagation
   - Implement error logging middleware

2. **Add Structured Logging**
   - Use `tracing` crate
   - Add request IDs for tracing
   - Log at appropriate levels (debug, info, warn, error)

3. **Improve Health Checks**
   - Add dependency health checks (model, database)
   - Add readiness vs liveness distinction
   - Add detailed status endpoint

#### Deliverables:
- Consistent error handling
- Structured logging
- Better observability

---

## Phase 4: Documentation & Release Prep (Agent 7-8)

### Agent 7: Documentation
**Priority:** MEDIUM  
**Estimated Work:** 2-3 hours

#### Tasks:
1. **Update README.md**
   - Quick start guide
   - Configuration options
   - API reference
   - Deployment instructions

2. **Add API Documentation**
   - OpenAPI/Swagger spec
   - Request/response examples
   - Error codes documentation

3. **Update AGENTS.md**
   - Document agent capabilities
   - Add testing procedures
   - Add deployment procedures

#### Deliverables:
- Complete README
- API documentation
- Updated AGENTS.md

---

### Agent 8: Release Preparation
**Priority:** HIGH  
**Estimated Work:** 2-3 hours

#### Tasks:
1. **Version Bump**
   - Update version to 0.8.1
   - Update CHANGELOG.md
   - Create git tag

2. **Pre-release Checklist**
   - Run all tests
   - Run clippy
   - Run security audit
   - Verify build on target platform

3. **CI/CD Updates**
   - Update GitHub Actions
   - Add security scanning
   - Add automated releases

#### Deliverables:
- Version 0.8.1 released
- Complete CHANGELOG
- Updated CI/CD

---

## Phase 5: Future Features (Optional - Agent 9+)

### Agent 9-10: v0.9.0 Features (If Roadmap is Serious)

**Note:** The v0.9.0 roadmap claims features not yet implemented. These agents would:

1. **sqlite-vec Integration**
   - Add native sqlite-vec extension
   - Implement SIMD-accelerated search

2. **Binary BLOB Storage**
   - Implement zero-copy deserialization
   - Add migration from JSON storage

3. **Matryoshka Embeddings**
   - Add dimension reduction
   - Implement two-tier search

---

## Execution Order & Context Passing

```
Agent 1 (Security)
    ↓ [shares: updated main.rs, security findings]
Agent 2 (Dead Code)  
    ↓ [shares: refactored codebase, clippy output]
Agent 3 (Performance)
    ↓ [shares: optimized search, new indexes]
Agent 4 (Config)
    ↓ [shares: config module, cleaned magic numbers]
Agent 5 (Testing)
    ↓ [shares: test suite, coverage report]
Agent 6 (Error Handling)
    ↓ [shares: error types, logging setup]
Agent 7 (Documentation)
    ↓ [shares: README, API docs]
Agent 8 (Release)
    ↓ [shares: v0.8.1 tag, CHANGELOG]
```

---

## Context Preservation Rules

1. **Each agent MUST:**
   - Read AGENTS.md before starting
   - Read previous agent's output summary
   - Update AGENTS.md with findings
   - Commit changes with conventional commit message

2. **Shared State:**
   - Use git branches: `fix/security`, `fix/dead-code`, `perf/optimization`, etc.
   - Merge to main after each agent completes
   - Tag releases after each phase

3. **Rate Limiting Avoidance:**
   - Agents work sequentially (not parallel)
   - Use appropriate timeouts
   - Batch git operations

---

## Success Criteria

| Phase | Criteria |
|-------|----------|
| Phase 1 | Zero security vulnerabilities, zero dead code |
| Phase 2 | Search <1ms P95, proper indexes |
| Phase 3 | 80%+ test coverage, structured logging |
| Phase 4 | Complete documentation, v0.8.1 released |

---

## Quick Start Commands

```bash
# Verify current state
cd services/brain-server
cargo clippy -- -D dead_code -D warnings
cargo test
cargo build --release

# Start development
git checkout -b fix/security
# ... do Agent 1 work ...
git add . && git commit -m "fix: resolve CORS security vulnerability"
git push origin fix/security
```

---

**Plan Version:** 1.0  
**Created:** 2026-02-24  
**Status:** Ready for Execution
