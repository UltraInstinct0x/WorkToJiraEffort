# WorkToJiraEffort Architecture Analysis

This directory contains comprehensive documentation of the application's architecture, including data flow, component responsibilities, and recommended LLM integration points.

## Documents

### 1. [COMPLETE ARCHITECTURE](01_COMPLETE_ARCHITECTURE.md) (Primary Document)
**Read this first for a deep understanding of the entire system**

Contents:
- Screenpipe data fetching and parsing
- Jira logging implementation
- Complete data flow from screenpipe to Jira
- Data processing and filtering logic
- Main components and their responsibilities
- Recommended LLM integration options
- Data structure transformations

**Key Sections:**
- Section 1: Screenpipe data fetching (lines 51-111 in screenpipe.rs)
- Section 2: Jira logging layer (jira.rs)
- Section 3: Data flow sequence
- Section 4: Processing pipeline
- Section 5: Component architecture
- Section 6: LLM integration recommendations

---

### 2. [LLM INJECTION POINTS](02_LLM_INJECTION_POINTS.md) (Implementation Guide)
**Read this to understand where and how to add LLM capabilities**

Contents:
- 3 primary injection points
- 1 optional advanced injection point
- Detailed code examples for each point
- Input/output data at each point
- Implementation strategy (4 phases)
- Technical design recommendations

**Quick Summary:**
- **Injection Point 1:** Activity Enrichment (post-consolidation)
- **Injection Point 2:** Smart Issue Detection (before find_issue_from_activity)
- **Injection Point 3:** Worklog Comment Enhancement (before log_work)
- **Injection Point 4:** Batch Analysis (optional, advanced)

**Recommended Starting Point:** Injection Point 2 (Smart Issue Detection)
- Lowest risk
- Highest impact
- Easiest to implement
- Can fail gracefully with regex fallback

---

### 3. [CODE LOCATIONS REFERENCE](03_CODE_LOCATIONS_REFERENCE.md) (Quick Reference)
**Use this to quickly find where to make changes**

Contents:
- All 7 source files with function/struct listing
- Line numbers for every major component
- Data transformation points with line references
- Configuration loading order
- Summary table of key code sections

**File Guide:**
- `screenpipe.rs` (122 lines): Screenpipe API client
- `screenpipe_manager.rs` (225 lines): Subprocess lifecycle
- `tracker.rs` (200 lines): Main orchestration
- `jira.rs` (110 lines): Jira integration
- `config.rs` (115 lines): Configuration management
- `daemon.rs` (119 lines): HTTP control API
- `main.rs` (142 lines): CLI entry point

---

### 4. [EXECUTIVE SUMMARY](04_EXECUTIVE_SUMMARY.md) (Overview)
**Read this for a quick understanding without deep technical details**

Contents:
- How the app works in simple terms
- 5 key data transformations
- Architecture diagram
- Component responsibilities table
- 5 current limitations and their LLM solutions
- Recommended integration strategy (4 phases)
- Success criteria for each phase

**Perfect For:**
- Understanding the big picture
- Explaining to non-technical stakeholders
- Planning LLM implementation phases
- Understanding what each component does

---

## Quick Navigation Guide

**I want to understand...**

- **The overall architecture** → Start with Executive Summary
- **Where to add LLM code** → Read LLM Injection Points
- **Exact file locations and line numbers** → Check Code Locations Reference
- **Everything in detail** → Read Complete Architecture

**I need to implement...**

- **Smart issue detection** → LLM Injection Points, Section 2 + Code Locations Reference
- **Comment enrichment** → LLM Injection Points, Section 3 + Code Locations Reference
- **Full activity enrichment** → Complete Architecture, Section 4 + LLM Injection Points, Section 1
- **Batch analysis** → LLM Injection Points, Section 4 (Advanced)

**I want to understand the data flow for...**

- **Screenpipe data fetching**: Complete Architecture Section 1 + Code Locations Reference
- **Jira logging**: Complete Architecture Section 2 + Code Locations Reference
- **Complete pipeline**: Complete Architecture Section 3 + LLM Injection Points
- **Processing & filtering**: Complete Architecture Section 4

---

## Key Findings Summary

### Current State
The application uses a simple pipeline:
1. Poll Screenpipe API every 5 minutes
2. Consolidate activities by app:window
3. Detect Jira issue keys via regex `[A-Z]+-\d+`
4. Log time to Jira and Salesforce

### The Problem
The current issue detection is **too simple**:
- Only finds issue keys in window titles
- Completely ignores the `Activity.description` field
- Misses issues mentioned in captured text
- Cannot classify work types
- Generates generic comments

### The Opportunity
Add LLM analysis to:
1. **Smart Issue Detection**: Find issues in descriptions (~2-3 days)
2. **Comment Enrichment**: Generate meaningful summaries (~1-2 days)
3. **Activity Classification**: Tag work types (~3-5 days)
4. **Batch Analysis**: Generate reports and trends (~5-7 days)

### Recommended Approach
Start with **Phase 1: Smart Issue Detection**
- Lowest risk (regex fallback)
- Highest immediate impact
- Can disable LLM easily
- Minimal code changes needed

---

## Architecture at a Glance

```
SCREENPIPE DATA INPUT (what app/window, timestamps, text)
        ↓
FETCH LAYER (screenpipe.rs: parse JSON, normalize)
        ↓
CONSOLIDATION (tracker.rs: deduplicate by app:window)
        ↓
FILTERING (tracker.rs: skip if < 60 seconds)
        ↓
DETECTION (jira.rs: find issue key via regex) ⚠️ SIMPLE
        ↓
ENRICHMENT (minimal - just format a comment)
        ↓
LOGGING (jira.rs + salesforce.rs: HTTP POST to APIs)
        ↓
OUTPUT TO JIRA & SALESFORCE
```

**Where LLM Fits:** Between Filtering and Logging, to enhance Detection and Enrichment layers

---

## File Organization

```
WorkToJiraEffort/
├── ARCHITECTURE_ANALYSIS/          ← You are here
│   ├── README.md                   (this file)
│   ├── 01_COMPLETE_ARCHITECTURE.md (detailed technical)
│   ├── 02_LLM_INJECTION_POINTS.md  (implementation guide)
│   ├── 03_CODE_LOCATIONS_REFERENCE.md (quick reference)
│   └── 04_EXECUTIVE_SUMMARY.md     (overview)
│
├── src/
│   ├── main.rs                     (CLI entry point)
│   ├── config.rs                   (configuration)
│   ├── screenpipe.rs               (Screenpipe API client)
│   ├── screenpipe_manager.rs       (subprocess management)
│   ├── tracker.rs                  (main orchestration)
│   ├── jira.rs                     (Jira integration) ← ADD LLM HERE
│   ├── salesforce.rs               (Salesforce integration)
│   ├── daemon.rs                   (HTTP control API)
│   └── llm.rs                      (NEW: LLM integration)
│
└── Cargo.toml                      (dependencies)
```

---

## Next Steps

1. **Read the Executive Summary** to understand the system
2. **Review LLM Injection Points** to see where LLM fits
3. **Check Code Locations Reference** for exact line numbers
4. **Read Complete Architecture** for deep understanding
5. **Start with Phase 1** of the recommended strategy

---

## Key Metrics

- **Project Size**: ~1,300 lines of Rust code
- **Main Loop**: Polls every 300 seconds (5 minutes)
- **API Integration Points**: 3 (Screenpipe, Jira, Salesforce)
- **Components**: 8 files
- **Data Model**: Simple Activity struct with 5 fields
- **Issue Detection**: Currently regex-only

---

## Questions This Analysis Answers

- Where does screenpipe data come in? → screenpipe.rs lines 51-111
- How is issue detection done? → jira.rs lines 79-93 (regex pattern matching)
- Where should LLM be added? → 4 options in LLM Injection Points
- What's the data transformation flow? → Complete Architecture Section 7
- How do I extend Activity struct? → screenpipe.rs lines 7-14
- How is Jira logging done? → jira.rs lines 36-77
- What config options exist? → config.rs, all sections
- How do I find a specific component? → Code Locations Reference

