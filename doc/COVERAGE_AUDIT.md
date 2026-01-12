# Test Coverage Audit Report

## Overview

**Current Coverage: 75.47%** (Target: 90%)

This document tracks test coverage improvements for Milestone 31, Phase 2.

---

## Coverage Summary by Layer

| Layer | Current | Target | Gap |
|-------|---------|--------|-----|
| Domain | ~85% | 90% | ~5% |
| Infrastructure | ~75% | 90% | ~15% |
| API Admin | ~60% | 90% | ~30% (improved from ~20%) |
| API V1 | ~50% | 90% | ~40% (improved from ~30%) |
| CLI/Config | 0% | N/A | Excluded |

---

## Modules Requiring Tests (Priority Order)

### Priority 1: API Admin Endpoints (0-30% coverage)

These are HTTP handlers - need unit tests for request validation and response mapping.

| Module | Coverage | Lines | Action |
|--------|----------|-------|--------|
| api\admin\knowledge_bases.rs | 4.77% | 880 | Add handler unit tests |
| api\admin\usage.rs | 7.36% | 462 | Add handler unit tests |
| api\admin\credentials.rs | 7.85% | 446 | Add handler unit tests |
| api\admin\api_keys.rs | 10.69% | 262 | Add handler unit tests |
| api\admin\test_cases.rs | 12.56% | 406 | Add handler unit tests |
| api\admin\prompts.rs | 14.81% | 189 | Add handler unit tests |
| api\admin\metrics.rs | 17.78% | 45 | Add middleware tests |
| api\admin\webhooks.rs | 18.83% | 239 | Add handler unit tests |
| api\admin\experiments.rs | 18.95% | 475 | Add handler unit tests |
| api\admin\models.rs | 21.64% | 573 | Add handler unit tests |
| api\admin\workflows.rs | 25.28% | 447 | Add handler unit tests |
| api\admin\external_apis.rs | 25.00% | 132 | Add handler unit tests |
| api\admin\teams.rs | 25.79% | 159 | Add handler unit tests |

### Priority 2: API V1 Endpoints (20-35% coverage)

| Module | Coverage | Lines | Action |
|--------|----------|-------|--------|
| api\v1\workflows.rs | 23.46% | 179 | Add handler unit tests |
| api\v1\models.rs | 32.14% | 56 | Add handler unit tests |
| api\v1\chat.rs | 30.32% | 696 | Add handler unit tests |
| api\v1\operations.rs | 47.95% | 73 | Add handler unit tests |

### Priority 3: Infrastructure (< 50% coverage)

| Module | Coverage | Lines | Action |
|--------|----------|-------|--------|
| infrastructure\cache\redis.rs | 7.41% | 297 | Add Redis cache tests (mock Redis) |
| infrastructure\storage\migrations\mod.rs | 22.17% | 212 | Migration tests excluded |
| infrastructure\storage\postgres.rs | 26.67% | 165 | Add Postgres tests (mock pool) |
| infrastructure\user\postgres_repository.rs | 26.53% | 245 | Add Postgres tests (mock pool) |
| infrastructure\observability\metrics.rs | 38.85% | 157 | Add metrics tests |
| infrastructure\observability\tracing_setup.rs | 4.64% | 151 | Tracing setup excluded |
| infrastructure\workflow\executor_impl.rs | 50.44% | 914 | Add executor tests |

### Priority 4: Domain Layer (< 75% coverage)

| Module | Coverage | Lines | Action |
|--------|----------|-------|--------|
| domain\test_case\repository.rs | 53.03% | 264 | Add repository tests |
| domain\usage\repository.rs | 54.50% | 200 | Add repository tests |
| domain\credentials\provider.rs | 61.54% | 26 | Add provider tests |
| domain\knowledge_base\filter.rs | 64.06% | 434 | Add filter tests |
| domain\llm\message.rs | 66.07% | 56 | Add message tests |
| domain\llm\response.rs | 67.80% | 59 | Add response tests |

---

## Excluded from Coverage Target

These modules are infrastructure/startup code that's difficult to unit test:

| Module | Reason |
|--------|--------|
| main.rs | Entry point |
| lib.rs | App initialization |
| cli\*.rs | CLI commands |
| config\app_config.rs | Config loading |
| api\router.rs | Router setup |
| api\state.rs | AppState trait defs |
| api\admin\mod.rs | Router aggregation |
| api\v1\mod.rs | Router aggregation |
| api\auth\mod.rs | Auth router |
| domain\chain\repository.rs | Trait definitions only |
| domain\traits\repository.rs | Trait definitions only |
| infrastructure\storage\migrations\mod.rs | DB migrations |
| infrastructure\observability\tracing_setup.rs | Tracing init |

---

## Testing Strategy

### API Handlers
For each admin/v1 endpoint module:
1. Test request deserialization/validation
2. Test response serialization
3. Test error handling
4. Mock service dependencies

### Infrastructure
1. Mock external dependencies (Redis, Postgres, HTTP)
2. Test business logic in isolation
3. Use in-memory alternatives for integration

### Domain
1. Test entity validation
2. Test business logic methods
3. Test error conditions

---

## Progress Tracking

### Phase 2a: API Admin Tests (COMPLETE - 274 new tests)
- [x] knowledge_bases.rs tests (38 new tests)
- [x] usage.rs tests (20 new tests)
- [x] credentials.rs tests (19 new tests)
- [x] api_keys.rs tests (23 new tests)
- [x] test_cases.rs tests (40 new tests)
- [x] prompts.rs tests (12 new tests)
- [x] webhooks.rs tests (16 new tests)
- [x] experiments.rs tests (19 new tests)
- [x] models.rs tests (17 new tests)
- [x] workflows.rs tests (19 new tests)
- [x] external_apis.rs tests (8 new tests)
- [x] teams.rs tests (12 new tests)
- [x] config.rs tests (25 new tests)
- [x] execution_logs.rs tests (26 new tests)

### Phase 2b: API V1 Tests (COMPLETE - 20 new tests)
- [x] chat.rs tests (4 existing tests)
- [x] workflows.rs tests (15 new tests)
- [x] models.rs tests (4 new tests)
- [x] operations.rs tests (9 new tests)

### Phase 2c: Infrastructure Tests
- [ ] redis.rs tests
- [ ] postgres.rs tests
- [ ] workflow executor tests

### Phase 2d: Domain Tests
- [ ] test_case repository tests
- [ ] usage repository tests
- [ ] filter builder tests

---

## High-Coverage Modules (>90%)

These modules are well-tested:

| Module | Coverage |
|--------|----------|
| domain\crag\config.rs | 100% |
| domain\embedding\request.rs | 100% |
| domain\error.rs | 100% |
| domain\experiment\assignment.rs | 100% |
| domain\operation\repository.rs | 100% |
| domain\semantic_cache\config.rs | 100% |
| domain\storage\entity.rs | 100% |
| domain\team\validation.rs | 100% |
| domain\user\validation.rs | 100% |
| domain\workflow\executor.rs | 100% |
| domain\workflow\repository.rs | 100% |
| infrastructure\crag\threshold_scorer.rs | 100% |
| infrastructure\cache\in_memory.rs | 98.23% |
| infrastructure\embedding\openai.rs | 97.20% |
| domain\crag\scorer.rs | 97.45% |
| domain\workflow\context.rs | 97.44% |
