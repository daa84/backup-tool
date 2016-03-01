
use chrono::UTC;
use std::time::Duration;

pub fn calc_time<A, R>(fun: fn(a: &A) -> Result<R, String>,
                       arg: &A)
                       -> Result<(R, Duration), (String, Duration)> {
    let start_time = UTC::now().timestamp();
    let res = fun(arg);
    let end_time = UTC::now().timestamp();
    match res {
        Ok(res) => Ok((res, Duration::from_secs((end_time - start_time) as u64))),
        Err(err) => Err((err, Duration::from_secs((end_time - start_time) as u64))),
    }
}

pub trait DurationFormatter {
    fn to_hhmmss(&self) -> String;
}

impl DurationFormatter for Duration {
    fn to_hhmmss(&self) -> String {
        let secs = self.as_secs();
        let hh = secs / (60 * 60);
        let mm = (secs / 60) % 60;
        let ss = secs % 60;
        format!("{:02}:{:02}:{:02}", hh, mm, ss)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_duration_format() {
        assert_eq!("01:01:01",
                   Duration::from_secs(60 * 60 + 60 + 1).to_hhmmss());
    }
}
