use super::{
    Identifier::{AlphaNumeric, Numeric},
    *,
};

// Tests adapted from the tests for Elixir's version module

macro_rules! assert_parse_test {
    ($name:ident, $input:expr, $major:expr, $minor:expr, $patch:expr,$pre:expr, $build:expr) => {
        #[test]
        fn $name() {
            assert_eq!(
                Version::parse($input).unwrap(),
                Version {
                    major: $major,
                    minor: $minor,
                    patch: $patch,
                    pre: $pre,
                    build: $build
                }
            );
        }
    };

    ($name:ident, $input:expr, $major:expr, $minor:expr, $patch:expr, $pre:expr) => {
        #[test]
        fn $name() {
            assert_eq!(
                Version::parse($input).unwrap(),
                Version {
                    major: $major,
                    minor: $minor,
                    patch: $patch,
                    pre: $pre,
                    build: None
                }
            );
        }
    };

    ($name:ident, $input:expr, $major:expr, $minor:expr, $patch:expr) => {
        #[test]
        fn $name() {
            assert_eq!(
                Version::parse($input).unwrap(),
                Version {
                    major: $major,
                    minor: $minor,
                    patch: $patch,
                    pre: vec![],
                    build: None
                }
            );
        }
    };
}

macro_rules! assert_parse_fail_test {
    ($name:ident, $input:expr) => {
        #[test]
        fn $name() {
            println!("{}", $input);
            Version::parse($input).unwrap_err();
        }
    };
}

assert_parse_test!(triplet, "1.2.3", 1, 2, 3);

assert_parse_test!(
    build,
    "1.4.5+ignore",
    1,
    4,
    5,
    vec![],
    Some("ignore".to_string())
);

assert_parse_test!(
    two_part_build,
    "0.0.1+sha.0702245",
    0,
    0,
    1,
    vec![],
    Some("sha.0702245".to_string())
);

assert_parse_test!(
    pre,
    "1.4.5-6-g3318bd5",
    1,
    4,
    5,
    vec![AlphaNumeric("6-g3318bd5".to_string())]
);

assert_parse_test!(
    multi_part_pre,
    "1.4.5-6.7.eight",
    1,
    4,
    5,
    vec![Numeric(6), Numeric(7), AlphaNumeric("eight".to_string())]
);

assert_parse_test!(
    pre_and_build,
    "1.4.5-6-g3318bd5+ignore",
    1,
    4,
    5,
    vec![AlphaNumeric("6-g3318bd5".to_string())],
    Some("ignore".to_string())
);

assert_parse_fail_test!(just_a_word, "foobar");

assert_parse_fail_test!(just_major, "2");

assert_parse_fail_test!(major_dor, "2.");

assert_parse_fail_test!(major_minor, "2.3");

assert_parse_fail_test!(major_minor_dot, "2.3.");

assert_parse_fail_test!(triplet_dash, "2.3.0-");

assert_parse_fail_test!(triplet_plus, "2.3.0+");

assert_parse_fail_test!(triplet_dot, "2.3.0.");

assert_parse_fail_test!(quad, "2.3.0.4");

assert_parse_fail_test!(missing_minor, "2.3.-rc.1");

assert_parse_fail_test!(missing_minor_with_dot, "2.3.+rc.1");

assert_parse_fail_test!(zero_pre, "2.3.0-01");

assert_parse_fail_test!(double_zero_pre, "2.3.00-1");

assert_parse_fail_test!(double_zero, "2.3.00");

assert_parse_fail_test!(leading_zero_minor, "2.03.0");

assert_parse_fail_test!(leading_zero_major, "02.3.0");

assert_parse_fail_test!(extra_whitespace, "0. 0.0");

assert_parse_fail_test!(and_in_version, "0.1.0-andpre");
