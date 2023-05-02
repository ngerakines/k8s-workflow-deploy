use chrono::{DateTime, Utc};
use tracing::warn;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) enum Supression {
    Range(DateTime<Utc>, DateTime<Utc>),
}

impl Supression {
    pub(crate) fn is_supressed(&self, time: DateTime<Utc>) -> bool {
        match self {
            Supression::Range(min, max) => time >= *min && time <= *max,
        }
    }

    pub(crate) fn key(&self) -> DateTime<Utc> {
        match self {
            Supression::Range(min, _) => *min,
        }
    }
}

pub(crate) fn parse_supressions(values: Vec<String>) -> Vec<Supression> {
    let mut supressions = Vec::new();
    for value in values.iter() {
        if let Some(parsed_value) = parse_supression(value) {
            supressions.push(parsed_value);
        }
    }
    supressions.sort();
    supressions.dedup_by(|a, b| a.key() == b.key());

    supressions
}

pub(crate) fn parse_supression(value: &str) -> Option<Supression> {
    let parts: Vec<&str> = value.split(' ').collect();
    match parts.len() {
        1 => match parts[0].parse::<DateTime<Utc>>() {
            Ok(parsed_date) => {
                let min = parsed_date.with_timezone(&Utc);
                let max: DateTime<Utc> = min + chrono::Duration::hours(1);
                Some(Supression::Range(min, max))
            }
            Err(err) => {
                warn!("Unable to parse supression: {} - {:?}", value, err);
                None
            }
        },
        2 => {
            match (
                parts[0].parse::<DateTime<Utc>>(),
                parts[1].parse::<DateTime<Utc>>(),
            ) {
                (Ok(min), Ok(max)) => {
                    let min = min.with_timezone(&Utc);
                    let max = max.with_timezone(&Utc);
                    if min > max {
                        warn!("min is greater than max: {:?} > {:?}", min, max);
                        return None;
                    }
                    Some(Supression::Range(min, max))
                }
                (Err(err), _) => {
                    warn!("Unable to parse first supression: {} - {:?}", parts[0], err);
                    None
                }
                (_, Err(err)) => {
                    warn!(
                        "Unable to parse second supression: {} - {:?}",
                        parts[1], err
                    );
                    None
                }
            }
        }
        _ => None,
    }
}
