//! Paged ("virtual") table view: only one window of rows is loaded at a time.

/// Rows loaded per fetch when browsing table data in the explorer.
pub const VIRTUAL_TABLE_PAGE_SIZE: u32 = 200;

#[derive(Debug, Clone)]
pub struct VirtualTableMeta {
    /// 0-based index of the first row in this window in the full table.
    pub window_start: u64,
    pub page_size: u32,
    pub loaded_count: usize,
    pub total_rows: Option<u64>,
    pub has_more_before: bool,
    pub has_more_after: bool,
}

impl VirtualTableMeta {
    #[must_use]
    pub fn from_fetch(
        window_start: u64,
        page_size: u32,
        loaded_row_count: usize,
        total_rows: Option<u64>,
    ) -> Self {
        let has_more_before = window_start > 0;
        let has_more_after = compute_has_more_after(
            window_start,
            page_size,
            loaded_row_count,
            total_rows,
        );
        Self {
            window_start,
            page_size,
            loaded_count: loaded_row_count,
            total_rows,
            has_more_before,
            has_more_after,
        }
    }

    /// Title suffix for the table panel (row range and paging hint).
    #[must_use]
    pub fn title_suffix(&self, filtered: bool, visible_rows: usize) -> String {
        if filtered {
            return format!(" ({visible_rows} matches · filter)");
        }
        if self.loaded_count == 0 {
            return match self.total_rows {
                Some(t) => format!(" (0 of {t} · j/k across pages)"),
                None => " (empty page · j/k across pages)".to_string(),
            };
        }
        let start = self.window_start + 1;
        let end = self.window_start + self.loaded_count as u64;
        match self.total_rows {
            Some(t) => format!(" ({start}-{end} of {t} · j/k across pages)"),
            None => format!(" ({start}-{end} · j/k across pages)"),
        }
    }
}

fn compute_has_more_after(
    window_start: u64,
    page_size: u32,
    loaded: usize,
    total: Option<u64>,
) -> bool {
    if loaded < page_size as usize {
        return false;
    }
    match total {
        Some(t) => (window_start + loaded as u64) < t,
        None => true,
    }
}
