# Plan 07 — card-ram Paging Layer

> **Phase:** 1 — Kernel Core
> **Prerequisites:** Plan 06 (Phase 0 complete — boot chain proven)
> **Estimated scope:** ~600 lines, hardware-agnostic kernel code using HAL sector I/O

---

## Objective

Implement virtual memory on hardware without an MMU. The card-ram paging layer manages a page table in SRAM, a frame pool in PSRAM (or SRAM on CYD), and a backing store in the 1GB raw MMC partition on the SD card.

## Architecture

```
Page Table (SRAM)         — metadata: which page maps to which sector, dirty bit, LRU tick
     ↕
Frame Pool (PSRAM/SRAM)   — resident working set: N × 4096-byte frames
     ↕ evict/load
MMC Partition (SD raw)    — 1GB backing store, 512-byte sectors
```

## Data Structures

### Page Table Entry (from designdoc §6.2)

```rust
struct PageEntry {
    sector: u32,      // absolute sector in MMC partition
    frame: Option<*mut u8>,  // pointer into frame pool, None if evicted
    dirty: bool,      // modified since last write-back?
    last_used: u32,   // tick counter for LRU eviction
}
```

### Configuration

| Constant | CYD Value | Production Value | Notes |
|----------|-----------|-----------------|-------|
| `PAGE_SIZE` | 4096 | 4096 | Aligns to flash erase blocks |
| `FRAME_POOL_COUNT` | 4 | 16 | CYD: 16KB SRAM. Production: 64KB PSRAM. |
| `MMC_SECTOR_SIZE` | 512 | 512 | Standard SD sector |
| `SECTORS_PER_PAGE` | 8 | 8 | 4096 / 512 |
| `MMC_SECTOR_COUNT` | 2,097,152 | 2,097,152 | 1GB ÷ 512 |

## Implementation Steps

### Core Paging Engine

- [ ] Define `PageTable` struct: array of `PageEntry`, frame pool allocation bitmap, global tick counter
- [ ] `page_alloc(n_pages) → PageId` — allocate N contiguous logical pages, assign backing sectors
- [ ] `page_read(page_id) → &[u8; PAGE_SIZE]` — ensure page is resident (load from MMC if evicted), update LRU, return frame pointer
- [ ] `page_write(page_id) → &mut [u8; PAGE_SIZE]` — same as read but marks dirty
- [ ] `page_free(page_id)` — release page, free frame and backing sector
- [ ] `flush_dirty()` — write all dirty frames to their backing sectors
- [ ] `evict_lru()` — find least-recently-used frame, write back if dirty, free frame for reuse

### LRU Eviction

- [ ] On every `page_read`/`page_write`: increment global tick, set `last_used = tick`
- [ ] When frame pool is full and a new page needs loading: evict the frame with lowest `last_used`
- [ ] If evicted frame is dirty: write back to MMC first

### Sector Allocator

- [ ] Simple bitmap or free-list for MMC sectors
- [ ] Sector 0 is reserved for page table checkpoint + hibernate header
- [ ] Allocate sectors sequentially from sector 1 onward
- [ ] Free sectors when pages are freed

### HAL Integration

- [ ] All MMC reads/writes go through `Platform::sd_read_sectors()` / `sd_write_sectors()`
- [ ] Paging layer is completely hardware-agnostic
- [ ] Frame pool memory allocated at init from PSRAM (or SRAM on CYD)

## Acceptance Criteria

- Allocate pages, write data, evict via LRU, read back — data intact
- Dirty pages are written back before eviction
- `flush_dirty()` persists all modified pages to MMC
- Frame pool never exceeds `FRAME_POOL_COUNT`
- Sector 0 is never allocated to a regular page

## Testing Strategy

- Unit tests with a mock `Platform` (in-memory sector storage)
- Stress test: allocate more pages than frames, write unique data to each, read all back — verify correctness despite evictions
- Verify LRU ordering: access pages in known pattern, confirm eviction order

## Notes

- This layer is the foundation for hibernate (Plan 08) and for loading app binaries larger than available RAM.
- The paging layer does NOT handle the hibernate header in sector 0 — that's Plan 08's responsibility.
- Performance: SD card sector I/O is slow (~1ms per sector). Minimize evictions. Consider read-ahead for sequential access patterns (future optimization, not Phase 1).
