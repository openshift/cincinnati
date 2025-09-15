use std::str::FromStr;

use language_tags::LanguageTag;

// All tests here may be completly nonsensical.

// Tests from RFC 5646 2.1.1
#[test]
fn test_formatting() {
    assert_eq!(
        "mn-Cyrl-MN",
        LanguageTag::parse("mn-Cyrl-MN").unwrap().as_str()
    );
    assert_eq!(
        "mn-Cyrl-MN",
        LanguageTag::parse("MN-cYRL-mn").unwrap().as_str()
    );
    assert_eq!(
        "mn-Cyrl-MN",
        LanguageTag::parse("mN-cYrL-Mn").unwrap().as_str()
    );
    assert_eq!(
        "en-CA-x-ca",
        LanguageTag::parse("en-CA-x-ca").unwrap().as_str()
    );
    assert_eq!(
        "sgn-BE-FR",
        LanguageTag::parse("sgn-BE-FR").unwrap().as_str()
    );
    assert_eq!(
        "az-Latn-x-latn",
        LanguageTag::parse("az-Latn-x-latn").unwrap().as_str()
    );
    assert_eq!("i-ami", LanguageTag::parse("i-ami").unwrap().as_str());
    assert_eq!("i-ami", LanguageTag::parse("I-AMI").unwrap().as_str());
    assert_eq!(
        "sl-afb-Latn-005-nedis",
        LanguageTag::parse("SL-AFB-lATN-005-nEdis")
            .unwrap()
            .as_str()
    )
}

// Tests from RFC 5646 2.2.1
#[test]
fn test_primary_language() {
    assert_eq!("fr", LanguageTag::parse("fr").unwrap().primary_language());
    assert_eq!("de", LanguageTag::parse("de").unwrap().primary_language());
    assert_eq!(
        "x-fr-ch",
        LanguageTag::parse("x-fr-CH").unwrap().primary_language()
    );
    assert_eq!(
        "i-klingon",
        LanguageTag::parse("i-klingon").unwrap().primary_language()
    );
    assert_eq!(
        "i-bnn",
        LanguageTag::parse("i-bnn").unwrap().primary_language()
    );
    assert_eq!(
        "zh-hakka",
        LanguageTag::parse("zh-hakka").unwrap().primary_language()
    )
}

// Tests from RFC 5646 2.2.2
#[test]
fn test_extended_language() {
    fn parts(tag: &LanguageTag) -> (&str, &str, Option<&str>, Vec<&str>) {
        (
            tag.full_language(),
            tag.primary_language(),
            tag.extended_language(),
            tag.extended_language_subtags().collect(),
        )
    }

    assert_eq!(("zh", "zh", None, vec![]), parts(&"zh".parse().unwrap()));
    assert_eq!(
        ("zh-gan", "zh", Some("gan"), vec!["gan"]),
        parts(&"zh-gan".parse().unwrap())
    );
    assert_eq!(
        ("zh-gan-foo", "zh", Some("gan-foo"), vec!["gan", "foo"]),
        parts(&"zh-gan-foo".parse().unwrap())
    );
    assert_eq!(
        ("zh-min-nan", "zh-min-nan", None, vec![]),
        parts(&"zh-min-nan".parse().unwrap())
    );
    assert_eq!(
        ("i-tsu", "i-tsu", None, vec![]),
        parts(&"i-tsu".parse().unwrap())
    );
    assert_eq!(("zh", "zh", None, vec![]), parts(&"zh-CN".parse().unwrap()));
    assert_eq!(
        ("zh-gan", "zh", Some("gan"), vec!["gan"]),
        parts(&"zh-gan-CN".parse().unwrap())
    );
    assert_eq!(
        ("ar-afb", "ar", Some("afb"), vec!["afb"]),
        parts(&"ar-afb".parse().unwrap())
    );
}

// Tests from RFC 5646 2.2.3
#[test]
fn test_script() {
    fn parts(tag: &LanguageTag) -> (&str, Option<&str>) {
        (tag.primary_language(), tag.script())
    }

    assert_eq!(("sr", Some("Latn")), parts(&"sr-Latn".parse().unwrap()));
    assert_eq!(("ar", Some("Latn")), parts(&"ar-afb-Latn".parse().unwrap()))
}

// Tests from RFC 5646 2.2.4
#[test]
fn test_region() {
    fn parts(tag: &LanguageTag) -> (&str, Option<&str>, Option<&str>) {
        (tag.primary_language(), tag.script(), tag.region())
    }

    assert_eq!(("de", None, Some("AT")), parts(&"de-AT".parse().unwrap()));
    assert_eq!(
        ("sr", Some("Latn"), Some("RS")),
        parts(&"sr-Latn-RS".parse().unwrap())
    );
    assert_eq!(("es", None, Some("419")), parts(&"es-419".parse().unwrap()));
    assert_eq!(("ar", None, Some("DE")), parts(&"ar-DE".parse().unwrap()));
    assert_eq!(("ar", None, Some("005")), parts(&"ar-005".parse().unwrap()));
}

// Tests from RFC 5646 2.2.5
#[test]
fn test_variant() {
    fn parts(tag: &LanguageTag) -> (&str, Option<&str>, Vec<&str>) {
        (
            tag.primary_language(),
            tag.variant(),
            tag.variant_subtags().collect(),
        )
    }

    assert_eq!(("sl", None, vec![]), parts(&"sl".parse().unwrap()));
    assert_eq!(
        ("sl", Some("nedis"), vec!["nedis"]),
        parts(&"sl-nedis".parse().unwrap())
    );
    assert_eq!(
        ("de", Some("1996"), vec!["1996"]),
        parts(&"de-CH-1996".parse().unwrap())
    );
    assert_eq!(
        ("art-lojban", None, vec![]),
        parts(&"art-lojban".parse().unwrap())
    );
}

// Tests from RFC 5646 2.2.6
#[test]
fn test_extension() {
    fn parts(tag: &LanguageTag) -> (&str, Option<&str>, Vec<(char, &str)>) {
        (
            tag.primary_language(),
            tag.extension(),
            tag.extension_subtags().collect(),
        )
    }

    assert_eq!(("en", None, vec![]), parts(&"en".parse().unwrap()));
    assert_eq!(
        ("en", Some("a-bbb"), vec![('a', "bbb")]),
        parts(&"en-a-bbb-x-a-ccc".parse().unwrap())
    );
    assert_eq!(
        (
            "en",
            Some("a-babble-b-warble"),
            vec![('a', "babble"), ('b', "warble")]
        ),
        parts(&"en-a-babble-b-warble".parse().unwrap())
    );
    assert_eq!(
        ("fr", Some("a-latn"), vec![('a', "latn")]),
        parts(&"fr-a-Latn".parse().unwrap())
    );
    assert_eq!(
        (
            "en",
            Some("r-extended-sequence"),
            vec![('r', "extended-sequence")]
        ),
        parts(
            &"en-Latn-GB-boont-r-extended-sequence-x-private"
                .parse()
                .unwrap()
        )
    );
    assert_eq!(
        ("en", Some("r-az-r-qt"), vec![('r', "az"), ('r', "qt")]),
        parts(&"en-r-az-r-qt".parse().unwrap())
    );
    assert_eq!(("i-tsu", None, vec![]), parts(&"i-tsu".parse().unwrap()));
}

// Tests from RFC 5646 2.2.7
#[test]
fn test_privateuse() {
    fn parts(tag: &LanguageTag) -> (&str, Option<&str>, Vec<&str>) {
        (
            tag.primary_language(),
            tag.private_use(),
            tag.private_use_subtags().collect(),
        )
    }

    assert_eq!(("en", None, vec![]), parts(&"en".parse().unwrap()));
    assert_eq!(
        ("en", Some("x-us"), vec!["us"]),
        parts(&"en-x-US".parse().unwrap())
    );
    assert_eq!(
        ("el", Some("x-koine"), vec!["koine"]),
        parts(&"el-x-koine".parse().unwrap())
    );
    assert_eq!(
        ("x-fr-ch", Some("x-fr-ch"), vec!["fr", "ch"]),
        parts(&"x-fr-ch".parse().unwrap())
    );
    assert_eq!(
        ("es", Some("x-foobar-at-007"), vec!["foobar", "at", "007"]),
        parts(&"es-x-foobar-AT-007".parse().unwrap())
    )
}

// Tests from RFC 5646 2.2.9
#[test]
fn test_is_valid() {
    let valid_tags = vec![
        "sr-Latn-RS",
        "zh-gan",
        "zh-Latn-wadegile",
        "en-unifon",
        "en-a-bbb-x-a-ccc",
        "ccd",
        "qra",
        "en-Qabx",
        "en-QU",
        "en-XD",
        "qqq-Latn-RS",
    ];
    for valid_tag in valid_tags {
        let validation = LanguageTag::parse(valid_tag).unwrap().validate();
        assert!(
            validation.is_ok(),
            "{} is considered invalid: {}",
            valid_tag,
            validation.err().unwrap()
        );
    }
}

#[test]
fn test_is_not_valid() {
    let invalid_tags = vec![
        "zzz-Latn-RS",
        "sr-Latq-RS",
        "sr-Latn-ZY",
        "de-gan",
        "zhb-gan",
        "zh-Hans-wadegile",
        "de-unifon",
        "ena-unifon",
        "de-DE-1901-aaaaa-1901",
        "en-a-bbb-a-ccc",
        "ab-c-abc-r-toto-c-abc",
    ];
    for invalid_tag in invalid_tags {
        assert!(
            !LanguageTag::parse(invalid_tag).unwrap().is_valid(),
            "{} is considered valid",
            invalid_tag
        );
    }
}

#[test]
fn test_fmt() {
    assert_eq!(
        "ar-arb-Latn-DE-nedis-foobar",
        LanguageTag::parse("ar-arb-Latn-DE-nedis-foobar")
            .unwrap()
            .to_string()
    );
    assert_eq!(
        "ar-arb-Latn-DE-nedis-foobar",
        LanguageTag::parse("ar-arb-latn-de-nedis-foobar")
            .unwrap()
            .to_string()
    );
    assert_eq!(
        "ar-arb-Latn-DE-nedis-foobar",
        LanguageTag::parse("AR-ARB-LATN-DE-NEDIS-FOOBAR")
            .unwrap()
            .to_string()
    );
    assert_eq!(
        "xx-z-foo-a-bar-f-spam-b-eggs",
        LanguageTag::parse("xx-z-foo-a-bar-F-spam-b-eggs")
            .unwrap()
            .to_string()
    );
    assert_eq!(
        "hkgnmerm-x-e5-zf-vddjcpz-1v6",
        LanguageTag::parse("HkgnmerM-x-e5-zf-VdDjcpz-1V6")
            .unwrap()
            .to_string()
    );
    assert_eq!(
        "mgxqa-Ywep-8lcw-7bvt-h-dp1md-0h7-0z3ir",
        LanguageTag::parse("MgxQa-ywEp-8lcW-7bvT-h-dP1Md-0h7-0Z3ir")
            .unwrap()
            .to_string()
    );
}

#[test]
fn test_unicode() {
    assert!(LanguageTag::parse("zh-x-Üńìcødê").is_err());
}

#[test]
fn test_cmp() {
    assert_eq!(
        LanguageTag::parse("dE-AraB-lY"),
        LanguageTag::parse("DE-aRaB-LY")
    );
    assert_ne!(LanguageTag::parse("zh"), LanguageTag::parse("zh-Latn"));
}

// Tests from RFC 5646 4.5
#[test]
fn test_canonicalize() {
    let conversion = vec![
        ("sgn-BE-FR", "sfb"),
        ("no-nyn", "nn"),
        ("i-klingon", "tlh"),
        ("zh-hak", "hak"),
        ("en-BU", "en-MM"),
        ("iw", "he"),
        ("en-ZR", "en-CD"),
        ("sh-yue", "yue"),
        ("zh-yue-Hant-HK", "yue-Hant-HK"),
        ("is-Latn", "is"),
        ("ja-Latn-heploc", "ja-Latn-alalc97"),
        ("sl-nedis-metelko-nedis", "sl-nedis-metelko"),
        ("ja-Latn-hepburn-heploc", "ja-Latn-hepburn-alalc97"),
        ("en-b-warble-a-babble", "en-a-babble-b-warble"),
        ("en-b-ccc-bbb-a-aaa-X-xyz", "en-a-aaa-b-ccc-bbb-x-xyz"),
        ("en-r-az-r-qt", "en-r-az-r-qt"),
        ("en-r-qt-r-az", "en-r-az-r-qt"),
    ];
    for (input, output) in conversion {
        let canonicalization = LanguageTag::parse(input).unwrap().canonicalize();
        assert!(
            canonicalization.is_ok(),
            "Canonicalization of {} failed: {}",
            input,
            canonicalization.err().unwrap()
        );
        assert_eq!(
            LanguageTag::parse(output).unwrap(),
            canonicalization.unwrap()
        );
    }
}

#[test]
fn test_canonicalize_fail() {
    let invalid_tags = vec!["zh-cmn-cpx"];
    for invalid_tag in invalid_tags {
        assert!(
            LanguageTag::parse(invalid_tag)
                .unwrap()
                .canonicalize()
                .is_err(),
            "{} canonicalization succeeded",
            invalid_tag
        );
    }
}

// http://www.langtag.net/test-suites/well-formed-tags.txt
#[test]
fn test_wellformed_tags() {
    let tags = vec![
        "fr",
        "fr-Latn",
        "fr-fra", // Extended tag
        "fr-Latn-FR",
        "fr-Latn-419",
        "fr-FR",
        "ax-TZ",     // Not in the registry, but well-formed
        "fr-shadok", // Variant
        "fr-y-myext-myext2",
        "fra-Latn", // ISO 639 can be 3-letters
        "fra",
        "fra-FX",
        "i-klingon", // grandfathered with singleton
        "I-kLINgon", // tags are case-insensitive...
        "no-bok",    // grandfathered without singleton
        "fr-Lat",    // Extended",
        "mn-Cyrl-MN",
        "mN-cYrL-Mn",
        "fr-Latn-CA",
        "en-US",
        "fr-Latn-CA",
        "i-enochian", // Grand fathered
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
        "ab-x-abc-x-abc",     // anything goes after x
        "ab-x-abc-a-a",       // ditto",
        "i-default",          // grandfathered",
        "i-klingon",          // grandfathered",
        "abcd-Latn",          // Language of 4 chars reserved for future use
        "AaBbCcDd-x-y-any-x", // Language of 5-8 chars, registered
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
        "xr-lxs-qut", // extlangS
        "xr-lqt-qu",  // extlang + region
        "xr-p-lze",   // Extension
    ];
    for tag in tags {
        let result = LanguageTag::from_str(tag);
        assert!(
            result.is_ok(),
            "{} should be considered well-formed but returned error {}",
            tag,
            result.err().unwrap()
        );
    }
}

#[test]
fn test_match() {
    let de_latn_de: LanguageTag = "de-Latn-DE".parse().unwrap();
    let de_de: LanguageTag = "de-DE".parse().unwrap();
    let de: LanguageTag = "de".parse().unwrap();
    assert!(de.matches(&de));
    assert!(de.matches(&de_de));
    assert!(de.matches(&de_latn_de));
    assert!(!de_de.matches(&de));
    assert!(de_de.matches(&de_de));
    assert!(de_de.matches(&de_latn_de));
    assert!(!de_latn_de.matches(&de));
    assert!(!de_latn_de.matches(&de_de));
    assert!(de_latn_de.matches(&de_latn_de));
}

// http://www.langtag.net/test-suites/broken-tags.txt
#[test]
fn test_broken_tags() {
    let tags = vec![
        "f",
        "f-Latn",
        "fr-Latn-F",
        "a-value",
        "tlh-a-b-foo",
        "i-notexist", // grandfathered but not registered: always invalid
        "abcdefghi-012345678",
        "ab-abc-abc-abc-abc",
        "ab-abcd-abc",
        "ab-ab-abc",
        "ab-123-abc",
        "a-Hant-ZH",
        "a1-Hant-ZH",
        "ab-abcde-abc",
        "ab-1abc-abc",
        "ab-ab-abcd",
        "ab-123-abcd",
        "ab-abcde-abcd",
        "ab-1abc-abcd",
        "ab-a-b",
        "ab-a-x",
        "ab--ab",
        "ab-abc-",
        "-ab-abc",
        "abcd-efg",
        "aabbccddE",
    ];
    for tag in tags {
        let result = LanguageTag::from_str(tag);
        assert!(
            result.is_err(),
            "{} should be considered not well-formed but returned result {:?}",
            tag,
            result.ok().unwrap()
        );
    }
}

// http://www.langtag.net/test-suites/valid-tags.txt
#[test]
fn test_valid_tags() {
    let tags = vec![
        "fr",
        "fr-Latn",
        //Not valid "fr-fra", // Extended tag
        "fr-Latn-FR",
        "fr-Latn-419",
        "fr-FR",
        "fr-y-myext-myext2",
        "apa-Latn", // ISO 639 can be 3-letters
        "apa",
        "apa-CA",
        "i-klingon", // grandfathered with singleton
        "no-bok",    // grandfathered without singleton
        //Not valid "fr-Lat",    // Extended
        "mn-Cyrl-MN",
        "mN-cYrL-Mn",
        "fr-Latn-CA",
        "en-US",
        "fr-Latn-CA",
        "i-enochian", // Grand fathered
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
        "ab-x-abc-x-abc", // anything goes after x
        "ab-x-abc-a-a",   // ditto
        "i-default",      // grandfathered
        "i-klingon",      // grandfathered
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
    ];
    for tag in tags {
        let result = LanguageTag::from_str(tag);
        assert!(
            result.is_ok(),
            "{} should be considered well-formed but returned error {}",
            tag,
            result.err().unwrap()
        );
        let tag = result.unwrap();
        let validation = tag.validate();
        assert!(
            validation.is_ok(),
            "{} should be considered valid but returned error {}",
            tag,
            validation.err().unwrap()
        );
        let canonicalization = tag.canonicalize();
        assert!(
            canonicalization.is_ok(),
            "{} canonicalization should not fail but returned error {}",
            tag,
            canonicalization.err().unwrap()
        );
    }
}

// http://www.langtag.net/test-suites/invalid-tags.txt
#[test]
fn test_invalid_tags() {
    let tags = vec![
        "en-a-bbb-a-ccc",        // 'a' appears twice, moved from broken_tags
        "ab-c-abc-r-toto-c-abc", // 'c' appears twice ", moved from broken_tags
        "ax-TZ",                 // Not in the registry, but well-formed
        "fra-Latn",              // ISO 639 can be 3-letters
        "fra",
        "fra-FX",
        "abcd-Latn",          // Language of 4 chars reserved for future use
        "AaBbCcDd-x-y-any-x", // Language of 5-8 chars, registered
        "zh-Latm-CN",         // Typo
        "de-DE-1902",         // Wrong variant
        "fr-shadok",          // Variant
    ];
    for tag in tags {
        let result = LanguageTag::from_str(tag);
        assert!(
            result.is_ok(),
            "{} should be considered well-formed but returned error {}",
            tag,
            result.err().unwrap()
        );
        let validation = result.unwrap().validate();
        assert!(validation.is_err(), "{} should be considered invalid", tag);
    }
}

#[test]
fn test_random_good_tags() {
    // http://unicode.org/repos/cldr/trunk/tools/java/org/unicode/cldr/util/data/langtagTest.txt
    let tags = vec![
        "zszLDm-sCVS-es-x-gn762vG-83-S-mlL",
        "IIJdFI-cfZv",
        "kbAxSgJ-685",
        "tbutP",
        "hDL-595",
        "dUf-iUjq-0hJ4P-5YkF-WD8fk",
        "FZAABA-FH",
        "xZ-lh-4QfM5z9J-1eG4-x-K-R6VPr2z",
        "Fyi",
        "SeI-DbaG",
        "ch-xwFn",
        "OeC-GPVI",
        "JLzvUSi",
        "Fxh-hLAs",
        "pKHzCP-sgaO-554",
        "eytqeW-hfgH-uQ",
        "ydn-zeOP-PR",
        "uoWmBM-yHCf-JE",
        "xwYem",
        "zie",
        "Re-wjSv-Ey-i-XE-E-JjWTEB8-f-DLSH-NVzLH-AtnFGWoH-SIDE",
        "Ri-063-c-u6v-ZfhkToTB-C-IFfmv-XT-j-rdyYFMhK-h-pY-D5-Oh6FqBhL-hcXt-v-WdpNx71-\
         K-c74m4-eBTT7-JdH7Q1Z",
        "ji",
        "IM-487",
        "EPZ-zwcB",
        "GauwEcwo",
        "kDEP",
        "FwDYt-TNvo",
        "ottqP-KLES-x-9-i9",
        "fcflR-grQQ",
        "TvFwdu-kYhs",
        "WE-336",
        "MgxQa-ywEp-8lcW-7bvT-h-dP1Md-0h7-0Z3ir-K-Srkm-kA-7LXM-Z-whb2MiO-2mNsvbLm-W3O\
         -4r-U-KceIxHdI-gvMVgUBV-2uRUni-J0-7C8yTK2",
        "Hyr-B-evMtVoB1-mtsVZf-vQMV-gM-I-rr-kvLzg-f-lAUK-Qb36Ne-Z-7eFzOD-mv6kKf-l-miZ\
         7U3-k-XDGtNQG",
        "ybrlCpzy",
        "PTow-w-cAQ51-8Xd6E-cumicgt-WpkZv3NY-q-ORYPRy-v-A4jL4A-iNEqQZZ-sjKn-W-N1F-pzy\
         c-xP5eWz-LmsCiCcZ",
        "ih-DlPR-PE",
        "Krf-362",
        "WzaD",
        "EPaOnB-gHHn",
        "XYta",
        "NZ-RgOO-tR",
        "at-FE",
        "Tpc-693",
        "YFp",
        "gRQrQULo",
        "pVomZ-585",
        "laSu-ZcAq-338",
        "gCW",
        "PydSwHRI-TYfF",
        "zKmWDD",
        "X-bCrL5RL",
        "HK",
        "YMKGcLY",
        "GDJ-nHYa-bw-X-ke-rohH5GfS-LdJKsGVe",
        "tfOxdau-yjge-489-a-oB-I8Csb-1ESaK1v-VFNz-N-FT-ZQyn-On2-I-hu-vaW3-jIQb-vg0U-h\
         Ul-h-dO6KuJqB-U-tde2L-P3gHUY-vnl5c-RyO-H-gK1-zDPu-VF1oeh8W-kGzzvBbW-yuAJZ",
        "LwDux",
        "Zl-072",
        "Ri-Ar",
        "vocMSwo-cJnr-288",
        "kUWq-gWfQ-794",
        "YyzqKL-273",
        "Xrw-ZHwH-841-9foT-ESSZF-6OqO-0knk-991U-9p3m-b-JhiV-0Kq7Y-h-cxphLb-cDlXUBOQ-X\
         -4Ti-jty94yPp",
        "en-GB-oed",
        "LEuZl-so",
        "HyvBvFi-cCAl-X-irMQA-Pzt-H",
        "uDbsrAA-304",
        "wTS",
        "IWXS",
        "XvDqNkSn-jRDR",
        "gX-Ycbb-iLphEks-AQ1aJ5",
        "FbSBz-VLcR-VL",
        "JYoVQOP-Iytp",
        "gDSoDGD-lq-v-7aFec-ag-k-Z4-0kgNxXC-7h",
        "Bjvoayy-029",
        "qSDJd",
        "qpbQov",
        "fYIll-516",
        "GfgLyfWE-EHtB",
        "Wc-ZMtk",
        "cgh-VEYK",
        "WRZs-AaFd-yQ",
        "eSb-CpsZ-788",
        "YVwFU",
        "JSsHiQhr-MpjT-381",
        "LuhtJIQi-JKYt",
        "vVTvS-RHcP",
        "SY",
        "fSf-EgvQfI-ktWoG-8X5z-63PW",
        "NOKcy",
        "OjJb-550",
        "KB",
        "qzKBv-zDKk-589",
        "Jr",
        "Acw-GPXf-088",
        "WAFSbos",
        "HkgnmerM-x-e5-zf-VdDjcpz-1V6",
        "UAfYflJU-uXDc-YV",
        "x-CHsHx-VDcOUAur-FqagDTx-H-V0e74R",
        "uZIAZ-Xmbh-pd",
    ];
    for tag in tags {
        let result = LanguageTag::from_str(tag);
        assert!(
            result.is_ok(),
            "{} should be considered well-formed but returned error {}",
            tag,
            result.err().unwrap()
        );
    }
}

#[test]
fn test_random_bad_tags() {
    // http://unicode.org/repos/cldr/trunk/tools/java/org/unicode/cldr/util/data/langtagTest.txt
    let tags = vec![
        "EdY-z_H791Xx6_m_kj",
        "qWt85_8S0-L_rbBDq0gl_m_O_zsAx_nRS",
        "VzyL2",
        "T_VFJq-L-0JWuH_u2_VW-hK-kbE",
        "u-t",
        "Q-f_ZVJXyc-doj_k-i",
        "JWB7gNa_K-5GB-25t_W-s-ZbGVwDu1-H3E",
        "b-2T-Qob_L-C9v_2CZxK86",
        "fQTpX_0_4Vg_L3L_g7VtALh2",
        "S-Z-E_J",
        "f6wsq-02_i-F",
        "9_GcUPq_G",
        "QjsIy_9-0-7_Dv2yPV09_D-JXWXM",
        "D_se-f-k",
        "ON47Wv1_2_W",
        "f-z-R_s-ha",
        "N3APeiw_195_Bx2-mM-pf-Z-Ip5lXWa-5r",
        "IRjxU-E_6kS_D_b1b_H",
        "NB-3-5-AyW_FQ-9hB-TrRJg3JV_3C",
        "yF-3a_V_FoJQAHeL_Z-Mc-u",
        "n_w_bbunOG_1-s-tJMT5je",
        "Q-AEWE_X",
        "57b1O_k_R6MU_sb",
        "hK_65J_i-o_SI-Y",
        "wB4B7u_5I2_I_NZPI",
        "J24Nb_q_d-zE",
        "v6-dHjJmvPS_IEb-x_A-O-i",
        "8_8_dl-ZgBr84u-P-E",
        "nIn-xD7EVhe_C",
        "5_N-6P_x7Of_Lo_6_YX_R",
        "0_46Oo0sZ-YNwiU8Wr_d-M-pg1OriV",
        "laiY-5",
        "K-8Mdd-j_ila0sSpo_aO8_J",
        "wNATtSL-Cp4_gPa_fD41_9z",
        "H_FGz5V8_n6rrcoz0_1O6d-kH-7-N",
        "wDOrnHU-odqJ_vWl",
        "gP_qO-I-jH",
        "h",
        "dJ0hX-o_csBykEhU-F",
        "L-Vf7_BV_eRJ5goSF_Kp",
        "y-oF-chnavU-H",
        "9FkG-8Q-8_v",
        "W_l_NDQqI-O_SFSAOVq",
        "kDG3fzXw",
        "t-nsSp-7-t-mUK2",
        "Yw-F",
        "1-S_3_l",
        "u-v_brn-Y",
        "4_ft_3ZPZC5lA_D",
        "n_dR-QodsqJnh_e",
        "Hwvt-bSwZwj_KL-hxg0m-3_hUG",
        "mQHzvcV-UL-o2O_1KhUJQo_G2_uryk3-a",
        "b-UTn33HF",
        "r-Ep-jY-aFM_N_H",
        "K-k-krEZ0gwD_k_ua-9dm3Oy-s_v",
        "XS_oS-p",
        "EIx_h-zf5",
        "p_z-0_i-omQCo3B",
        "1_q0N_jo_9",
        "0Ai-6-S",
        "L-LZEp_HtW",
        "Zj-A4JD_2A5Aj7_b-m3",
        "x",
        "p-qPuXQpp_d-jeKifB-c-7_G-X",
        "X94cvJ_A",
        "F2D25R_qk_W-w_Okf_kx",
        "rc-f",
        "D",
        "gD_WrDfxmF-wu-E-U4t",
        "Z_BN9O4_D9-D_0E_KnCwZF-84b-19",
        "T-8_g-u-0_E",
        "lXTtys9j_X_A_m-vtNiNMw_X_b-C6Nr",
        "V_Ps-4Y-S",
        "X5wGEA",
        "mIbHFf_ALu4_Jo1Z1",
        "ET-TacYx_c",
        "Z-Lm5cAP_ri88-d_q_fi8-x",
        "rTi2ah-4j_j_4AlxTs6m_8-g9zqncIf-N5",
        "FBaLB85_u-0NxhAy-ZU_9c",
        "x_j_l-5_aV95_s_tY_jp4",
        "PL768_D-m7jNWjfD-Nl_7qvb_bs_8_Vg",
        "9-yOc-gbh",
        "6DYxZ_SL-S_Ye",
        "ZCa-U-muib-6-d-f_oEh_O",
        "Qt-S-o8340F_f_aGax-c-jbV0gfK_p",
        "WE_SzOI_OGuoBDk-gDp",
        "cs-Y_9",
        "m1_uj",
        "Y-ob_PT",
        "li-B",
        "f-2-7-9m_f8den_J_T_d",
        "p-Os0dua-H_o-u",
        "L",
        "rby-w",
    ];
    for tag in tags {
        let result = LanguageTag::from_str(tag);
        assert!(
            result.is_err(),
            "{} should be considered not well-formed but returned result {:?}",
            tag,
            result.ok().unwrap()
        );
    }
}
