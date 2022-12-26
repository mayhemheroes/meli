/*
 * melib - sieve module
 *
 * Copyright 2022 Manos Pitsidianakis
 *
 * This file is part of meli.
 *
 * meli is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * meli is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with meli. If not, see <http://www.gnu.org/licenses/>.
 */

use crate::parsec::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleBlock(pub Vec<Rule>);

/*
   MATCH-TYPE =/ COUNT / VALUE

  COUNT = ":count" relational-match

  VALUE = ":value" relational-match

  relational-match = DQUOTE
          ("gt" / "ge" / "lt" / "le" / "eq" / "ne") DQUOTE
          ; "gt" means "greater than", the C operator ">".
          ; "ge" means "greater than or equal", the C operator ">=".
          ; "lt" means "less than", the C operator "<".
          ; "le" means "less than or equal", the C operator "<=".
          ; "eq" means "equal to", the C operator "==".
          ; "ne" means "not equal to", the C operator "!=".
*/
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Rule {
    Block(RuleBlock),
    Stop,
    Keep,
    Discard,
    If {
        condition: (ConditionRule, RuleBlock),
        elsif: Option<(ConditionRule, RuleBlock)>,
        else_: Option<RuleBlock>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressOperator {
    All,
    Localpart,
    Domain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegerOperator {
    Over,
    Under,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// RFC 5231 Sieve Email Filtering: Relational Extension
pub enum RelationalMatch {
    /// "gt" means "greater than", the C operator ">".
    Gt,
    /// "ge" means "greater than or equal", the C operator ">=".
    Ge,
    /// "lt" means "less than", the C operator "<".
    Lt,
    /// "le" means "less than or equal", the C operator "<=".
    Le,
    /// "eq" means "equal to", the C operator "==".
    Eq,
    /// "ne" means "not equal to", the C operator "!=".
    Ne,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchOperator {
    Is,
    Matches,
    Contains,
    Count(RelationalMatch),
    Value(RelationalMatch),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterOperator {
    /// i;octet,
    Octet,
    ///i;ascii-casemap
    AsciiCasemap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneRule {
    /// "year"      => the year, "0000" .. "9999".
    Year,
    /// "month"     => the month, "01" .. "12".
    Month,
    /// "day"       => the day, "01" .. "31".
    Day,
    /// "date"      => the date in "yyyy-mm-dd" format.
    Date,
    /// "julian"    => the Modified Julian Day, that is, the date
    /// expressed as an integer number of days since
    /// 00:00 UTC on November 17, 1858 (using the Gregorian
    /// calendar).  This corresponds to the regular
    /// Julian Day minus 2400000.5.  Sample routines to
    /// convert to and from modified Julian dates are
    /// given in Appendix A.
    Julian,
    /// "hour"      => the hour, "00" .. "23".
    Hour,
    /// "minute"    => the minute, "00" .. "59".
    Minute,
    /// "second"    => the second, "00" .. "60".
    Second,
    /// "time"      => the time in "hh:mm:ss" format.
    Time,
    /// "iso8601"   => the date and time in restricted ISO 8601 format.
    Iso8601,
    /// "std11"     => the date and time in a format appropriate
    /// for use in a Date: header field [RFC2822].
    Std11,
    /// "zone"      => the time zone in use.  If the user specified a
    ///time zone with ":zone", "zone" will
    ///contain that value.  If :originalzone is specified
    ///this value will be the original zone specified
    ///in the date-time value.  If neither argument is
    ///specified the value will be the server's default
    ///time zone in offset format "+hhmm" or "-hhmm".  An
    ///offset of 0 (Zulu) always has a positive sign.
    Zone,
    /// "weekday"   => the day of the week expressed as an integer between "0" and "6". "0" is Sunday, "1" is Monday, etc.
    Weekday,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionRule {
    /// Logical OR operation.
    AnyOf(Vec<ConditionRule>),
    /// Logical AND operation.
    AllOf(Vec<ConditionRule>),
    /// Header values exist.
    Exists(Vec<String>),
    Header {
        comparator: Option<CharacterOperator>,
        match_operator: Option<MatchOperator>,
        header_names: Vec<String>,
        key_list: Vec<String>,
    },
    Date {
        comparator: Option<CharacterOperator>,
        match_type: Option<MatchOperator>,
        zone: ZoneRule,
        header_name: String,
        date_part: String,
        key_list: Vec<String>,
    },
    Address {
        comparator: Option<CharacterOperator>,
        address_part: Option<AddressOperator>,
        match_type: Option<MatchOperator>,
        header_list: Vec<String>,
        key_list: Vec<String>,
    },
    Not(Box<ConditionRule>),
    Size {
        operator: IntegerOperator,
        limit: u64,
    },
    Literal(bool),
}

pub mod parser {
    use super::*;
    macro_rules! parse_action {
        ($lit:literal, $action:expr, $parser_name:ident) => {
            pub fn $parser_name<'a>() -> impl Parser<'a, Rule> {
                move |input| {
                    map(
                        eat(right(match_literal_anycase($lit), eat(match_literal(";")))),
                        |_| $action,
                    )
                    .parse(input)
                }
            }
        };
    }

    parse_action! { "stop", Rule::Stop, parse_sieve_stop }
    parse_action! { "keep", Rule::Keep, parse_sieve_keep }
    parse_action! { "discard", Rule::Discard, parse_sieve_discard }

    #[inline(always)]
    pub fn token<'a>(literal: &'static str) -> impl Parser<'a, ()> {
        move |input| map(eat(match_literal_anycase(literal)), |_| ()).parse(input)
    }

    #[inline(always)]
    fn eat_inner<'a>() -> impl Parser<'a, ()> {
        move |input: &'a str| {
            let mut offset = 0;
            let input_b = input.as_bytes();
            while offset < input_b.len() {
                while offset < input_b.len()
                    && [b' ', b'\t', b'\n', b'\r'].contains(&input_b[offset])
                {
                    offset += 1;
                }
                if offset >= input_b.len() {
                    break;
                }
                if input_b[offset] == b'#' {
                    while offset < input_b.len()
                        && !input[offset..].starts_with("\r\n")
                        && !input[offset..].starts_with('\n')
                    {
                        offset += 1;
                    }
                    if offset >= input_b.len() {
                        break;
                    }
                    if input[offset..].starts_with("\r\n") {
                        offset += 2;
                    } else if input[offset..].starts_with('\n') {
                        offset += 1;
                    }
                } else if input[offset..].starts_with("/*") {
                    while offset < input_b.len() && !input[offset..].starts_with("*/") {
                        offset += 1;
                    }
                    if offset >= input_b.len() {
                        break;
                    }
                    if input[offset..].starts_with("*/") {
                        offset += 2;
                    }
                } else {
                    break;
                }
            }
            Ok((&input[offset..], ()))
        }
    }

    pub fn eat<'a, P, A>(parser: P) -> impl Parser<'a, A>
    where
        P: Parser<'a, A>,
    {
        move |input1| {
            let (input2, ()) = eat_inner().parse(input1)?;
            let (input3, res) = parser.parse(input2)?;
            let (input4, ()) = eat_inner().parse(input3)?;
            Ok((input4, res))
        }
    }

    // string       = quoted-string / multi-line
    //
    // quoted-other       = "\" octet-not-qspecial
    //                        ; represents just the octet-no-qspecial
    //                        ; character.  SHOULD NOT be used

    // quoted-safe        = CRLF / octet-not-qspecial
    //                        ; either a CRLF pair, OR a single octet other
    //                        ; than NUL, CR, LF, double-quote, or backslash

    // quoted-special     = "\" (DQUOTE / "\")
    //                        ; represents just a double-quote or backslash

    // quoted-string      = DQUOTE quoted-text DQUOTE

    // quoted-text        = *(quoted-safe / quoted-special / quoted-other)

    pub fn string<'a>() -> impl Parser<'a, String> {
        #[inline(always)]
        fn quoted_text<'a>() -> impl Parser<'a, String> {
            move |input: &'a str| {
                let mut offset = 0;
                let mut unescape_dquote: bool = false;
                let mut unescape_slash: bool = false;
                while offset < input.len() {
                    if input.len() >= offset + 2 {
                        if input.starts_with("\r\n") {
                            offset += 2;
                        } else if input.starts_with("\\\"") {
                            unescape_dquote = true;
                            offset += 2;
                        } else if input.starts_with("\\\\") {
                            unescape_slash = true;
                            offset += 2;
                        }
                    }
                    // a single octet other ; than NUL, CR, LF, double-quote, or backslash
                    if [b'\x00', b'\r', b'\n', b'"', b'\\'].contains(&input.as_bytes()[offset]) {
                        break;
                    }
                    offset += 1;
                }
                match (unescape_dquote, unescape_slash) {
                    (false, false) => Ok((&input[offset..], input[..offset].to_string())),
                    (true, false) => Ok((&input[offset..], input[..offset].replace("\\\"", "\""))),
                    (false, true) => Ok((&input[offset..], input[..offset].replace("\\\\", "\\"))),
                    (true, true) => Ok((
                        &input[offset..],
                        input[..offset].replace("\\\"", "\"").replace("\\\\", "\\"),
                    )),
                }
            }
        }

        #[inline(always)]
        fn quoted_string<'a>() -> impl Parser<'a, String> {
            delimited(token("\""), quoted_text(), token("\""))
        }

        //fn multiline() -> impl Parser<'a, String> {}
        //either(quoted_string(), multiline())
        quoted_string()
    }

    #[inline(always)]
    pub fn literal_map<'a, T: Clone>(literal: &'static str, value: T) -> impl Parser<'a, T> {
        move |input| map(token(literal), |_| value.clone()).parse(input)
    }

    // number             = 1*DIGIT [ QUANTIFIER ]
    // QUANTIFIER         = "K" / "M" / "G"
    pub fn number<'a>() -> impl Parser<'a, u64> {
        map_res(
            pair(
                is_a(b"0123456789"),
                pred(any_char, |c| {
                    ['k', 'm', 'g'].contains(&c.to_ascii_lowercase())
                }),
            ),
            |(num_s, quant)| {
                Ok(match (num_s.parse::<u64>(), quant.to_ascii_lowercase()) {
                    (Ok(num), 'k') => num * 1000,
                    (Ok(num), 'm') => num * 1000_000,
                    (Ok(num), 'g') => num * 1000_000_000,
                    _ => return Err(num_s),
                })
            },
        )
    }

    pub fn parse_sieve_integer_operator<'a>() -> impl Parser<'a, (IntegerOperator, u64)> {
        move |input| {
            eat(pair(
                either(
                    literal_map(":over", IntegerOperator::Over),
                    literal_map(":under", IntegerOperator::Under),
                ),
                eat(number()),
            ))
            .parse(input)
        }
    }
    // ":comparator" <comparator-name: string>
    pub fn parse_sieve_comparator<'a>() -> impl Parser<'a, CharacterOperator> {
        move |input| {
            eat(right(
                token(":comparator"),
                eat(map_res(string(), |s| {
                    if s == "i;octet" {
                        Ok(CharacterOperator::Octet)
                    } else if s == "i;ascii-casemap" {
                        Ok(CharacterOperator::AsciiCasemap)
                    } else {
                        Err("invalid comparator")
                    }
                })),
            ))
            .parse(input)
        }
    }

    // MATCH-TYPE   = ":is" / ":contains" / ":matches"
    pub fn parse_sieve_match_type<'a>() -> impl Parser<'a, MatchOperator> {
        move |input| {
            either(
                map(token(":is"), |_| MatchOperator::Is),
                either(
                    map(token(":contains"), |_| MatchOperator::Contains),
                    map(token(":matches"), |_| MatchOperator::Matches),
                ),
            )
            .parse(input)
        }
    }

    /* string-list  = "[" string *("," string) "]" / string
                    ; if there is only a single string, the brackets
                    ; are optional
    */
    pub fn string_list<'a>() -> impl Parser<'a, Vec<String>> {
        move |input| {
            either(
                delimited(
                    eat(token("[")),
                    separated_list0(string(), eat(token(",")), false),
                    eat(token("]")),
                ),
                map(string(), |s| vec![s]),
            )
            .parse(input)
        }
    }

    /* Usage:   header [COMPARATOR] [MATCH-TYPE]
     * <header-names: string-list> <key-list: string-list>
     */
    pub fn parse_sieve_header<'a>() -> impl Parser<'a, ConditionRule> {
        move |input| {
            map(
                eat(pair(
                    right(token("header"), move |input| {
                        crate::permutation! {
                            input,
                            comparator, Option<CharacterOperator>, opt(parse_sieve_comparator()),
                            match_type, Option<MatchOperator>, opt(parse_sieve_match_type())
                        }
                    }),
                    pair(eat(string_list()), eat(string_list())),
                )),
                |((comparator, match_operator), (header_names, key_list))| ConditionRule::Header {
                    comparator,
                    match_operator,
                    header_names,
                    key_list,
                },
            )
            .parse(input)
        }
    }

    // ADDRESS-PART = ":localpart" / ":domain" / ":all"
    pub fn parse_sieve_address_type<'a>() -> impl Parser<'a, AddressOperator> {
        move |input| {
            either(
                map(token(":localpart"), |_| AddressOperator::Localpart),
                either(
                    map(token(":domain"), |_| AddressOperator::Domain),
                    map(token(":all"), |_| AddressOperator::All),
                ),
            )
            .parse(input)
        }
    }

    // address [COMPARATOR] [ADDRESS-PART] [MATCH-TYPE] <header-list: string-list> <key-list: string-list>
    pub fn parse_sieve_address<'a>() -> impl Parser<'a, ConditionRule> {
        move |input| {
            map(
                eat(pair(
                    right(token("address"), move |input| {
                        crate::permutation! {
                            input,
                            match_type, Option<MatchOperator>, opt(parse_sieve_match_type()),
                            comparator, Option<CharacterOperator>, opt(parse_sieve_comparator()),
                            address_type, Option<AddressOperator>, opt(parse_sieve_address_type())
                        }
                    }),
                    pair(eat(string_list()), eat(string_list())),
                )),
                |((match_type, comparator, address_part), (header_list, key_list))| {
                    ConditionRule::Address {
                        comparator,
                        address_part,
                        match_type,
                        header_list,
                        key_list,
                    }
                },
            )
            .parse(input)
        }
    }

    pub fn parse_sieve_test<'a>() -> impl Parser<'a, ConditionRule> {
        move |input| {
            either(
                either(
                    literal_map("true", ConditionRule::Literal(true)),
                    literal_map("false", ConditionRule::Literal(false)),
                ),
                either(
                    either(
                        map(right(eat(token("exists")), eat(string_list())), |l| {
                            ConditionRule::Exists(l)
                        }),
                        map(
                            right(eat(token("size")), eat(parse_sieve_integer_operator())),
                            |(operator, limit)| ConditionRule::Size { operator, limit },
                        ),
                    ),
                    either(
                        either(
                            map(right(eat(token("not")), parse_sieve_test()), |cond| {
                                ConditionRule::Not(Box::new(cond))
                            }),
                            either(parse_sieve_header(), parse_sieve_address()),
                        ),
                        either(
                            map(right(eat(token("allof")), test_list()), |l| {
                                ConditionRule::AllOf(l)
                            }),
                            map(right(eat(token("anyof")), test_list()), |l| {
                                ConditionRule::AnyOf(l)
                            }),
                        ),
                    ),
                ),
            )
            .parse(input)
        }
    }

    /* test-list  = "(" test *("," test) ")"
     */
    pub fn test_list<'a>() -> impl Parser<'a, Vec<ConditionRule>> {
        move |input| {
            delimited(
                eat(token("(")),
                separated_list0(eat(parse_sieve_test()), eat(token(",")), false),
                eat(token(")")),
            )
            .parse(input)
        }
    }

    pub fn parse_sieve_rule<'a>() -> impl Parser<'a, Rule> {
        either(
            parse_sieve_stop(),
            either(
                parse_sieve_keep(),
                either(parse_sieve_discard(), parse_sieve_if()),
            ),
        )
    }

    pub fn parse_sieve_block<'a>() -> impl Parser<'a, RuleBlock> {
        move |input| {
            map(
                eat(delimited(
                    token("{"),
                    eat(zero_or_more(parse_sieve_rule())),
                    token("}"),
                )),
                |v| RuleBlock(v),
            )
            .parse(input)
        }
    }

    pub fn parse_sieve_if<'a>() -> impl Parser<'a, Rule> {
        either(
            map(
                pair(
                    parse_sieve_if_bare(),
                    eat(right(token("else"), eat(parse_sieve_block()))),
                ),
                |(ifbare, else_)| match ifbare {
                    Rule::If {
                        condition,
                        elsif,
                        else_: _,
                    } => Rule::If {
                        condition,
                        elsif,
                        else_: Some(else_),
                    },
                    _ => unreachable!(),
                },
            ),
            parse_sieve_if_bare(),
        )
    }

    pub fn parse_sieve_if_bare<'a>() -> impl Parser<'a, Rule> {
        either(
            map(
                pair(
                    eat(pair(
                        eat(right(token("if"), eat(parse_sieve_test()))),
                        eat(parse_sieve_block()),
                    )),
                    eat(pair(
                        eat(right(token("elsif"), eat(parse_sieve_test()))),
                        eat(parse_sieve_block()),
                    )),
                ),
                |(condition, elsif)| Rule::If {
                    condition,
                    elsif: Some(elsif),
                    else_: None,
                },
            ),
            map(
                pair(
                    eat(right(token("if"), eat(parse_sieve_test()))),
                    eat(parse_sieve_block()),
                ),
                |(cond, block)| Rule::If {
                    condition: (cond, block),
                    elsif: None,
                    else_: None,
                },
            ),
        )
    }

    pub fn parse_sieve<'a>() -> impl Parser<'a, Vec<Rule>> {
        eat(zero_or_more(eat(parse_sieve_rule())))
    }
}

#[cfg(test)]
mod test {
    use super::parser::*;
    use crate::parsec::Parser;

    use super::AddressOperator::*;
    use super::CharacterOperator::*;
    use super::ConditionRule::*;
    use super::IntegerOperator::*;
    use super::MatchOperator::*;
    use super::Rule::*;
    use super::RuleBlock;

    #[test]
    fn test_sieve_parse_strings() {
        assert_eq!(
            Ok(("", vec!["fileinto".to_string(), "reject".to_string()])),
            string_list().parse(r#"["fileinto", "reject"]"#)
        );

        assert_eq!(
            Ok(("", vec!["fileinto".to_string()])),
            string_list().parse(r#""fileinto""#)
        );
    }

    #[test]
    fn test_sieve_parse_conditionals() {
        /* Operators that start with : like :matches are unordered and optional, since they have
         * defaults. But that means we must handle any order correctly, which is tricky if we use
         * an optional parser; for an optional parser both None and Some(_) are valid values.
         */

        /* Permutations of two */
        let raw_input = r#"header :contains :comparator "i;octet" "Subject"
                   "MAKE MONEY FAST""#;
        let (_, first) = parse_sieve_test().parse(raw_input).unwrap();
        assert_eq!(
            Header {
                comparator: Some(Octet),
                match_operator: Some(Contains),
                header_names: ["Subject".to_string()].to_vec(),
                key_list: ["MAKE MONEY FAST".to_string()].to_vec()
            },
            first
        );

        let raw_input = r#"header :comparator "i;octet" :contains "Subject"
                   "MAKE MONEY FAST""#;
        assert_eq!(Ok(("", first)), parse_sieve_test().parse(raw_input));

        /* Permutations of three */
        let raw_input = r#"address :DOMAIN :comparator "i;octet" :is ["From", "To"] "example.com""#;
        let (_, first) = parse_sieve_test().parse(raw_input).unwrap();

        assert_eq!(
            &Address {
                comparator: Some(Octet),
                address_part: Some(Domain),
                match_type: Some(Is),
                header_list: ["From".to_string(), "To".to_string()].to_vec(),
                key_list: ["example.com".to_string()].to_vec()
            },
            &first
        );

        let raw_input =
            r#"address :DOMAIN :is :comparator "i;octet"  ["From", "To"] "example.com""#;
        assert_eq!(Ok(("", first.clone())), parse_sieve_test().parse(raw_input));

        let raw_input =
            r#"address :is :DOMAIN :comparator "i;octet"  ["From", "To"] "example.com""#;
        assert_eq!(Ok(("", first.clone())), parse_sieve_test().parse(raw_input));

        let raw_input = r#"address :is :comparator "i;octet" :DOMAIN ["From", "To"] "example.com""#;
        assert_eq!(Ok(("", first)), parse_sieve_test().parse(raw_input));
    }

    #[test]
    fn test_sieve_parse_ifs() {
        let raw_input = "if true {\nstop ;\n}";
        assert_eq!(
            Ok((
                "",
                If {
                    condition: (Literal(true), RuleBlock([Stop].to_vec())),
                    elsif: None,
                    else_: None
                }
            )),
            parse_sieve_rule().parse(raw_input)
        );

        let raw_input = r#"# Reject all messages that contain the string "viagra"in the Subject.
if header :contains "subject" "viagra"
{
    discard;
} else {
    keep;
}"#;

        assert_eq!(
            Ok((
                "",
                [If {
                    condition: (
                        Header {
                            comparator: None,
                            match_operator: Some(Contains),
                            header_names: ["subject".to_string()].to_vec(),
                            key_list: ["viagra".to_string()].to_vec()
                        },
                        RuleBlock([Discard].to_vec())
                    ),
                    elsif: None,
                    else_: Some(RuleBlock([Keep].to_vec()))
                }]
                .to_vec()
            )),
            parse_sieve().parse(raw_input)
        );

        let raw_input = r#"# Reject all messages that contain the string "viagra"in the Subject.
if header :contains "subject" "viagra"
{
    discard;
}
# Silently discard all messages sent from the tax man
elsif address :matches :domain "from" "*hmrc.gov.uk"
{
    keep;
}"#;
        assert_eq!(
            Ok((
                "",
                [If {
                    condition: (
                        Header {
                            comparator: None,
                            match_operator: Some(Contains),
                            header_names: ["subject".to_string()].to_vec(),
                            key_list: ["viagra".to_string()].to_vec()
                        },
                        RuleBlock([Discard].to_vec())
                    ),
                    elsif: Some((
                        Address {
                            comparator: None,
                            address_part: Some(Domain),
                            match_type: Some(Matches),
                            header_list: ["from".to_string()].to_vec(),
                            key_list: ["*hmrc.gov.uk".to_string()].to_vec()
                        },
                        RuleBlock([Keep].to_vec())
                    )),
                    else_: None
                }]
                .to_vec()
            )),
            parse_sieve().parse(raw_input)
        );
    }

    #[test]
    fn test_sieve_parse() {
        let raw_input = r#"# The hash character starts a one-line comment.

"#;
        assert_eq!(Ok(("", vec![])), parse_sieve().parse(raw_input));

        let raw_input = r#"# The hash character starts a one-line comment.
# Everything after a # character until the end of line is ignored.

/* this is a bracketed (C-style) comment. This type of comment can stretch
 * over many lines. A bracketed comment begins with a forward slash, followed
 * by an asterisk and ends with the inverse sequence: an asterisk followed
 * by a forward slash. */
"#;

        assert_eq!(Ok(("", vec![])), parse_sieve().parse(raw_input));
        // Test Lists (allof, anyof)

        let raw_input = r#"# This test checks against Spamassassin's header fields:
# If the spam level ls 4 or more and the Subject contains too
# many illegal characters, then silently discard the mail.
if allof (header :contains "X-Spam-Level" "****",
          header :contains "X-Spam-Report" "FROM_ILLEGAL_CHARS")
{
    discard;
}
# Discard mails that do not have a Date: or From: header field
# or mails that are sent from the marketing department at example.com.
elsif anyof (not exists ["from", "date"],
        header :contains "from" "marketing@example.com") {
    discard;
}"#;

        assert_eq!(
            Ok((
                "",
                [If {
                    condition: (
                        AllOf(
                            [
                                Header {
                                    comparator: None,
                                    match_operator: Some(Contains),
                                    header_names: ["X-Spam-Level".to_string()].to_vec(),
                                    key_list: ["****".to_string()].to_vec()
                                },
                                Header {
                                    comparator: None,
                                    match_operator: Some(Contains),
                                    header_names: ["X-Spam-Report".to_string()].to_vec(),
                                    key_list: ["FROM_ILLEGAL_CHARS".to_string()].to_vec()
                                }
                            ]
                            .to_vec()
                        ),
                        RuleBlock([Discard].to_vec())
                    ),
                    elsif: Some((
                        AnyOf(
                            [
                                Not(Box::new(Exists(
                                    ["from".to_string(), "date".to_string()].to_vec()
                                ))),
                                Header {
                                    comparator: None,
                                    match_operator: Some(Contains),
                                    header_names: ["from".to_string()].to_vec(),
                                    key_list: ["marketing@example.com".to_string()].to_vec()
                                }
                            ]
                            .to_vec()
                        ),
                        RuleBlock([Discard].to_vec())
                    )),
                    else_: None
                }]
                .to_vec()
            )),
            parse_sieve().parse(raw_input)
        );
        // Filter on message size
        let raw_input = r#"# Delete messages greater than half a MB
if size :over 500K
{
    discard;
}
# Also delete small mails, under 1k
if size :under 1k
{
    discard;
}"#;
        assert_eq!(
            Ok((
                "",
                [
                    If {
                        condition: (
                            Size {
                                operator: Over,
                                limit: 500000
                            },
                            RuleBlock([Discard].to_vec())
                        ),
                        elsif: None,
                        else_: None
                    },
                    If {
                        condition: (
                            Size {
                                operator: Under,
                                limit: 1000
                            },
                            RuleBlock([Discard].to_vec())
                        ),
                        elsif: None,
                        else_: None
                    }
                ]
                .to_vec()
            )),
            parse_sieve().parse(raw_input)
        );
    }
}
