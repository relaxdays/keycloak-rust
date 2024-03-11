macro_rules! paginate_api {
    (|$first:ident, $max:ident| {$api_call:expr}) => {{
        const PAGE_MAX: i32 = 100;

        let mut results = Vec::new();
        let $max = PAGE_MAX;
        let mut page_offset = Some(0);
        while let Some($first) = page_offset.take() {
            let page = $api_call;
            let page_len = page.len();
            results.extend(page.into_iter());
            // next page
            if page_len == PAGE_MAX as usize {
                tracing::trace!(
                    "increasing pagination offset (new page start={})",
                    $first + PAGE_MAX
                );
                page_offset = Some($first + PAGE_MAX);
            }
        }
        results
    }};
}
