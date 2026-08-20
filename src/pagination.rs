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
#[non_exhaustive]
pub struct PaginationMeta {
    /// `X-Pagination-Current-Page`
    pub current_page: Option<u32>,
    /// `X-Pagination-Page-Count`
    pub page_count: Option<u32>,
    /// `X-Pagination-Per-Page`
    pub per_page: Option<u32>,
    /// `X-Pagination-Total-Count`
    pub total_count: Option<u32>,
    /// `X-Rate-Limit-Limit` — the total request quota for the current
    /// window, when present on the response.
    pub rate_limit: Option<u32>,
    /// `X-Rate-Limit-Remaining` — requests left in the current window, when
    /// present on the response. Useful for self-throttling before hitting a
    /// `429`; see [`crate::Error::is_rate_limited`]/[`crate::Error::retry_after`]
    /// for the reactive (post-429) case.
    pub rate_limit_remaining: Option<u32>,
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
            rate_limit: parse(headers, "x-rate-limit-limit"),
            rate_limit_remaining: parse(headers, "x-rate-limit-remaining"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(
                reqwest::header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                v.parse().unwrap(),
            );
        }
        h
    }

    #[test]
    fn from_headers_parses_pagination_and_rate_limit_headers() {
        let meta = PaginationMeta::from_headers(&headers(&[
            ("x-pagination-current-page", "2"),
            ("x-pagination-page-count", "5"),
            ("x-pagination-per-page", "20"),
            ("x-pagination-total-count", "97"),
            ("x-rate-limit-limit", "120"),
            ("x-rate-limit-remaining", "106"),
        ]));
        assert_eq!(meta.current_page, Some(2));
        assert_eq!(meta.page_count, Some(5));
        assert_eq!(meta.rate_limit, Some(120));
        assert_eq!(meta.rate_limit_remaining, Some(106));
        assert!(meta.has_more());
        assert_eq!(meta.next_page(), Some(3));
    }

    #[test]
    fn next_page_and_has_more_are_none_on_last_page_or_missing_headers() {
        let last = PaginationMeta {
            current_page: Some(5),
            page_count: Some(5),
            ..Default::default()
        };
        assert_eq!(last.next_page(), None);
        assert!(!last.has_more());

        let missing = PaginationMeta::from_headers(&headers(&[]));
        assert_eq!(missing.next_page(), None);
        assert!(!missing.has_more());
        assert_eq!(missing.rate_limit, None);
    }
}
