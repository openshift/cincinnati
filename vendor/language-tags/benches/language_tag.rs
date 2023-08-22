use bencher::{benchmark_group, benchmark_main, black_box, Bencher};
use language_tags::LanguageTag;

fn parse_well_formed_tags(bench: &mut Bencher) {
    let tags = vec![
        // http://www.langtag.net/test-suites/well-formed-tags.txt
        "fr",
        "fr-Latn",
        "fr-fra",
        "fr-Latn-FR",
        "fr-Latn-419",
        "fr-FR",
        "ax-TZ",
        "fr-shadok",
        "fr-y-myext-myext2",
        "fra-Latn",
        "fra",
        "fra-FX",
        "i-klingon",
        "I-kLINgon",
        "no-bok",
        "fr-Lat",
        "mn-Cyrl-MN",
        "mN-cYrL-Mn",
        "fr-Latn-CA",
        "en-US",
        "fr-Latn-CA",
        "i-enochian",
        "x-fr-CH",
        "sr-Latn-CS",
        "es-419",
        "sl-nedis",
        "de-CH-1996",
        "de-Latg-1996",
        "sl-IT-nedis",
        "en-a-bbb-x-a-ccc",
        "de-a-value",
        "en-Latn-GB-boont-r-extended-sequence-x-private",
        "en-x-US",
        "az-Arab-x-AZE-derbend",
        "es-Latn-CO-x-private",
        "en-US-boont",
        "ab-x-abc-x-abc",
        "ab-x-abc-a-a",
        "i-default",
        "i-klingon",
        "abcd-Latn",
        "AaBbCcDd-x-y-any-x",
        "en",
        "de-AT",
        "es-419",
        "de-CH-1901",
        "sr-Cyrl",
        "sr-Cyrl-CS",
        "sl-Latn-IT-rozaj",
        "en-US-x-twain",
        "zh-cmn",
        "zh-cmn-Hant",
        "zh-cmn-Hant-HK",
        "zh-gan",
        "zh-yue-Hant-HK",
        "xr-lxs-qut",
        "xr-lqt-qu",
        "xr-p-lze",
    ];
    bench.bytes = tags.iter().map(|tag| tag.len() as u64).sum();
    bench.iter(|| {
        for tag in &tags {
            let _ = black_box(LanguageTag::parse(tag).unwrap());
        }
    });
}

fn validate_tags(bench: &mut Bencher) {
    let tags = vec![
        // http://www.langtag.net/test-suites/valid-tags.txt
        "fr",
        "fr-Latn",
        "fr-fra",
        "fr-Latn-FR",
        "fr-Latn-419",
        "fr-FR",
        "fr-y-myext-myext2",
        "apa-Latn",
        "apa",
        "apa-CA",
        "i-klingon",
        "no-bok",
        "fr-Lat",
        "mn-Cyrl-MN",
        "mN-cYrL-Mn",
        "fr-Latn-CA",
        "en-US",
        "fr-Latn-CA",
        "i-enochian",
        "x-fr-CH",
        "sr-Latn-CS",
        "es-419",
        "sl-nedis",
        "de-CH-1996",
        "de-Latg-1996",
        "sl-IT-nedis",
        "en-a-bbb-x-a-ccc",
        "de-a-value",
        "en-x-US",
        "az-Arab-x-AZE-derbend",
        "es-Latn-CO-x-private",
        "ab-x-abc-x-abc",
        "ab-x-abc-a-a",
        "i-default",
        "i-klingon",
        "en",
        "de-AT",
        "es-419",
        "de-CH-1901",
        "sr-Cyrl",
        "sr-Cyrl-CS",
        "sl-Latn-IT-rozaj",
        "en-US-x-twain",
        "zh-cmn",
        "zh-cmn-Hant",
        "zh-cmn-Hant-HK",
        "zh-gan",
        "zh-yue-Hant-HK",
        "en-Latn-GB-boont-r-extended-sequence-x-private",
        "en-US-boont",
        // http://www.langtag.net/test-suites/invalid-tags.txt
        "en-a-bbb-a-ccc",
        "ab-c-abc-r-toto-c-abc",
        "ax-TZ",
        "fra-Latn",
        "fra",
        "fra-FX",
        "abcd-Latn",
        "AaBbCcDd-x-y-any-x",
        "zh-Latm-CN",
        "de-DE-1902",
        "fr-shadok",
    ];
    bench.bytes = tags.iter().map(|tag| tag.len() as u64).sum();
    let tags: Vec<_> = tags
        .into_iter()
        .map(|t| LanguageTag::parse(t).unwrap())
        .collect();
    bench.iter(|| {
        for tag in &tags {
            let _ = black_box(tag.is_valid());
        }
    });
}

fn canonicalize_tags(bench: &mut Bencher) {
    let tags = vec![
        // http://www.langtag.net/test-suites/valid-tags.txt
        "fr",
        "fr-Latn",
        "fr-fra",
        "fr-Latn-FR",
        "fr-Latn-419",
        "fr-FR",
        "fr-y-myext-myext2",
        "apa-Latn",
        "apa",
        "apa-CA",
        "i-klingon",
        "no-bok",
        "fr-Lat",
        "mn-Cyrl-MN",
        "mN-cYrL-Mn",
        "fr-Latn-CA",
        "en-US",
        "fr-Latn-CA",
        "i-enochian",
        "x-fr-CH",
        "sr-Latn-CS",
        "es-419",
        "sl-nedis",
        "de-CH-1996",
        "de-Latg-1996",
        "sl-IT-nedis",
        "en-a-bbb-x-a-ccc",
        "de-a-value",
        "en-x-US",
        "az-Arab-x-AZE-derbend",
        "es-Latn-CO-x-private",
        "ab-x-abc-x-abc",
        "ab-x-abc-a-a",
        "i-default",
        "i-klingon",
        "en",
        "de-AT",
        "es-419",
        "de-CH-1901",
        "sr-Cyrl",
        "sr-Cyrl-CS",
        "sl-Latn-IT-rozaj",
        "en-US-x-twain",
        "zh-cmn",
        "zh-cmn-Hant",
        "zh-cmn-Hant-HK",
        "zh-gan",
        "zh-yue-Hant-HK",
        "en-Latn-GB-boont-r-extended-sequence-x-private",
        "en-US-boont",
        // http://www.langtag.net/test-suites/invalid-tags.txt
        "en-a-bbb-a-ccc",
        "ab-c-abc-r-toto-c-abc",
        "ax-TZ",
        "fra-Latn",
        "fra",
        "fra-FX",
        "abcd-Latn",
        "AaBbCcDd-x-y-any-x",
        "zh-Latm-CN",
        "de-DE-1902",
        "fr-shadok",
    ];
    bench.bytes = tags.iter().map(|tag| tag.len() as u64).sum();
    let tags: Vec<_> = tags
        .into_iter()
        .map(|t| LanguageTag::parse(t).unwrap())
        .collect();
    bench.iter(|| {
        for tag in &tags {
            let _ = black_box(tag.canonicalize());
        }
    });
}

benchmark_group!(
    benches,
    parse_well_formed_tags,
    validate_tags,
    canonicalize_tags
);
benchmark_main!(benches);
