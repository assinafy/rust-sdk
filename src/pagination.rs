//! Pagination types.

use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};

/// A page of results, combining the decoded items with pagination metadata
/// extracted from the server's `X-Pagination-*` headers.
#[derive(Debug, Clone)]
pub struct Page<T> {
    /// Items in this page.
    pub data: Vec<T>,
    /// Pagination metadata extracted from response headers.
    pub meta: PaginationMeta,
}

impl<T> Page<T> {
    /// Returns true if more pages are available.
    pub fn has_more(&self) -> bool {
        self.meta.has_more()
    }

    /// Returns the next page number, if any.
    pub fn next_page(&self) -> Option<u32> {
        self.meta.next_page()
    }

    /// Convert a page of `T` into a page of `U`.
    pub fn map<U, F: FnMut(T) -> U>(self, f: F) -> Page<U> {
        Page {
            data: self.data.into_iter().map(f).collect(),
            meta: self.meta,
        }
    }
}

impl<T> IntoIterator for Page<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

/// Pagination metadata reported by the API via `X-Pagination-*` response
/// headers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaginationMeta {
    /// `X-Pagination-Current-Page`
    pub current_page: Option<u32>,
    /// `X-Pagination-Page-Count`
    pub page_count: Option<u32>,
    /// `X-Pagination-Per-Page`
    pub per_page: Option<u32>,
    /// `X-Pagination-Total-Count`
    pub total_count: Option<u32>,
}

impl PaginationMeta {
    /// Returns the next page number if `current_page < page_count`.
    pub fn next_page(&self) -> Option<u32> {
        match (self.current_page, self.page_count) {
            (Some(cur), Some(total)) if cur < total => Some(cur + 1),
            _ => None,
        }
    }

    /// Returns true if more pages are available.
    pub fn has_more(&self) -> bool {
        self.next_page().is_some()
    }

    pub(crate) fn from_headers(headers: &HeaderMap) -> Self {
        fn parse(headers: &HeaderMap, name: &str) -> Option<u32> {
            headers
                .get(name)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok())
        }
        PaginationMeta {
            current_page: parse(headers, "x-pagination-current-page"),
            page_count: parse(headers, "x-pagination-page-count"),
            per_page: parse(headers, "x-pagination-per-page"),
            total_count: parse(headers, "x-pagination-total-count"),
        }
    }
}
