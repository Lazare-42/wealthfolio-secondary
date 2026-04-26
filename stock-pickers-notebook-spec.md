# Stock Picker's Notebook - Wealthfolio Addon

## Project Overview

A Wealthfolio addon that enhances the existing Holdings page by bringing investment thesis tracking and analysis directly into the portfolio tracker. This addon extends—rather than replaces—the current interface by adding strategy classification, conviction scoring, thesis notes, and valuation targets seamlessly into the existing UI.

**Key Design Principle**: Enhance existing UI patterns rather than building new interfaces from scratch. Integrate naturally with the Holdings page's allocation charts and detail views.

## Problem Statement

Currently, deep analysis, thesis work, and conviction scores happen outside portfolio trackers—scattered across Excel sheets, notepads, and documents. This critical qualitative information should live alongside the quantitative portfolio data for better decision-making and performance review.

## Implementation Strategy

This addon will:
1. **Extend the Holdings Insights view** with a new "Strategy Allocation" chart (alongside Country, Sector, etc.)
2. **Store thesis data** using addon-scoped storage (keyed by `instrument.id`)
3. **Add detail sheets** for thesis editing when clicking holdings
4. **Optionally provide a dedicated addon page** for comprehensive thesis management

## Core Features

### 1. Strategy Classification Column

**Purpose:** Categorize each holding by investment strategy

**Options:**
- Value
- Deep Value
- Growth
- GARP (Growth at Reasonable Price)
- Dividend
- Momentum
- Speculative
- Custom (user-defined)

**Functionality:**
- Dropdown selection per holding
- Strategy allocation pie chart showing portfolio composition by strategy
- Filter holdings by strategy
- Strategy weight calculation (by portfolio value)

### 2. Conviction Score

**Purpose:** Capture confidence level in each investment

**Scale:** 1-5 or Low/Medium/High

**Features:**
- Visual badge/indicator (color-coded)
- Sort/filter by conviction level
- Track conviction changes over time
- Average conviction score across portfolio

### 3. Thesis Notes (Time-stamped)

**Purpose:** Document and track evolution of investment thesis

**Features:**
- Rich text editor for detailed notes
- Automatic timestamping on every edit
- Version history view showing thesis evolution
- Key metrics tracking fields:
  - Metrics to watch (user-defined)
  - Current status/standing
  - Entry/exit triggers
- Search across all thesis notes
- Export individual or all theses

**Data Structure:**
```json
{
  "timestamp": "2025-01-15T10:30:00Z",
  "content": "Strong moat in SaaS market. Target: $150/share...",
  "editedBy": "user"
}
```

### 4. Valuation Fields

**Purpose:** Track target prices and entry zones

**Fields:**
- **Target Valuation** (user input, $)
- **Buy Price Area** (range or single value, $)
- **Current Price vs Target** (auto-calculated %)
- **Distance to Buy Zone** (auto-calculated %)

**Visual Indicators:**
- Green: At or below buy price area
- Yellow: Within 10% of buy price area
- Red: Significantly above buy price area

### 5. Enhanced Holdings View

**Integrated Table Columns:**
- Symbol
- Current Price
- Position Size
- **Strategy** (new)
- **Conviction** (new)
- **Thesis Preview** (new - first 100 chars)
- **Target Price** (new)
- **Buy Zone** (new)
- **Actions** (edit thesis, view history)

## User Interface

### Main View
- Enhanced holdings table with new columns
- Strategy allocation pie chart (top of page)
- Conviction score distribution chart
- Filters: Strategy, Conviction, "Needs Review"

### Thesis Editor Modal
- Full-screen or large dialog
- Rich text editor
- Timestamp display
- History timeline sidebar
- Save/Cancel buttons
- "Mark for Review" flag

### Charts Dashboard
- Strategy allocation pie chart
- Conviction score distribution
- Holdings above/below target price
- Thesis age distribution (staleness indicator)

## Technical Specifications

### Permissions Required
```json
{
  "permissions": [
    "holdings:read",
    "storage:read",
    "storage:write"
  ]
}
```

### Data Storage

**Storage Keys:**
- `analysis:{symbol}` - Individual stock analysis
- `notebook:settings` - User preferences
- `notebook:strategies` - Custom strategy definitions

**Data Model:**
```typescript
interface StockAnalysis {
  symbol: string;
  strategy: string;
  conviction: 1 | 2 | 3 | 4 | 5;
  targetValuation: number | null;
  buyPriceArea: {
    low: number | null;
    high: number | null;
  };
  thesisHistory: ThesisEntry[];
  metricsToWatch: string[];
  lastReviewed: string;
  needsReview: boolean;
}

interface ThesisEntry {
  timestamp: string;
  content: string;
  version: number;
}
```

### Dependencies
- `@wealthfolio/addon-sdk` - Core addon APIs
- `@wealthfolio/ui` - UI components
- `recharts` - Charts (already available)
- `lucide-react` - Icons (already available)
- `date-fns` - Timestamp formatting (already available)

## Implementation Phases

### Phase 1: MVP (Week 1-2)
- [ ] Basic holdings table integration
- [ ] Strategy selection dropdown
- [ ] Conviction score input
- [ ] Simple thesis text field (no history)
- [ ] Strategy pie chart
- [ ] Local storage implementation

### Phase 2: Enhanced Features (Week 3-4)
- [ ] Timestamped thesis editing
- [ ] Thesis history view
- [ ] Target price/buy zone fields
- [ ] Price comparison indicators
- [ ] Filter/sort functionality

### Phase 3: Advanced Analytics (Week 5-6)
- [ ] Conviction distribution chart
- [ ] Thesis staleness warnings
- [ ] Export functionality
- [ ] Search across theses
- [ ] Custom strategy definitions

### Phase 4: Polish & Publishing (Week 7)
- [ ] UI/UX refinement
- [ ] Documentation
- [ ] Example data/tutorial
- [ ] Addon marketplace submission

## Success Metrics

- User can track thesis for 100% of holdings
- Reduce time spent context-switching to external tools
- Enable better decision-making through historical thesis review
- Positive community feedback on addon marketplace

## Future Enhancements (Post v1.0)

- Attach external links/documents to thesis
- Collaborative thesis editing (multi-user)
- AI-powered thesis summarization
- Integration with news/earnings calendar
- Email/notification reminders to review stale theses
- Compare thesis vs actual performance (was my thesis correct?)

## Open Questions

1. Should thesis notes support markdown formatting?
2. Maximum thesis history entries to store (performance)?
3. Should strategy categories be customizable from day 1?
4. Export format preferences (Markdown, PDF, CSV)?

---

**Version:** 1.0
**Author:** [Your Name]
**Date:** 2025-01-21
**License:** MIT
