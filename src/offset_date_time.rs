use core::cmp::Ordering;
#[cfg(feature = "std")]
use core::convert::From;
#[cfg(feature = "formatting")]
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::{Add, AddAssign, Sub, SubAssign};
use core::time::Duration as StdDuration;
#[cfg(feature = "formatting")]
use std::io;
#[cfg(feature = "std")]
use std::time::SystemTime;

#[cfg(feature = "formatting")]
use crate::formatting::formattable::sealed::Formattable;
#[cfg(feature = "parsing")]
use crate::parsing::parsable::sealed::Parsable;
use crate::{error, Date, Duration, PrimitiveDateTime, Time, UtcOffset, Weekday};

/// The Julian day of the Unix epoch.
const UNIX_EPOCH_JULIAN_DAY: i32 = Date::__from_ordinal_date_unchecked(1970, 1).to_julian_day();

/// A [`PrimitiveDateTime`] with a [`UtcOffset`].
///
/// All comparisons are performed using the UTC time.
// Internally, an `OffsetDateTime` is a thin wrapper around a [`PrimitiveDateTime`] coupled with a
// [`UtcOffset`]. This offset is added to the date, time, or datetime as necessary for presentation
// or returning from a function.
#[derive(Debug, Clone, Copy, Eq)]
pub struct OffsetDateTime {
    /// The [`PrimitiveDateTime`], which is _always_ UTC.
    pub(crate) utc_datetime: PrimitiveDateTime,
    /// The [`UtcOffset`], which will be added to the [`PrimitiveDateTime`] as necessary.
    pub(crate) offset: UtcOffset,
}

impl OffsetDateTime {
    /// Midnight, 1 January, 1970 (UTC).
    ///
    /// ```rust
    /// # use time::{OffsetDateTime, macros::datetime};
    /// assert_eq!(OffsetDateTime::UNIX_EPOCH, datetime!("1970-01-01 0:00 UTC"),);
    /// ```
    pub const UNIX_EPOCH: Self = Date::__from_ordinal_date_unchecked(1970, 1)
        .midnight()
        .assume_utc();

    // region: now
    /// Create a new `OffsetDateTime` with the current date and time in UTC.
    ///
    /// ```rust
    /// # use time::{OffsetDateTime, macros::offset};
    /// assert!(OffsetDateTime::now_utc().year() >= 2019);
    /// assert_eq!(OffsetDateTime::now_utc().offset(), offset!("UTC"));
    /// ```
    #[cfg(feature = "std")]
    #[cfg_attr(__time_03_docs, doc(cfg(feature = "std")))]
    pub fn now_utc() -> Self {
        SystemTime::now().into()
    }

    /// Attempt to create a new `OffsetDateTime` with the current date and time in the local offset.
    /// If the offset cannot be determined, an error is returned.
    ///
    /// ```rust
    /// # use time::OffsetDateTime;
    /// # if false {
    /// assert!(OffsetDateTime::now_local().is_ok());
    /// # }
    /// ```
    ///
    /// Due to a [soundness bug](https://github.com/time-rs/time/issues/293),
    /// the error value is currently always returned on Unix-like platforms.
    #[cfg(feature = "local-offset")]
    #[cfg_attr(__time_03_docs, doc(cfg(feature = "local-offset")))]
    pub fn now_local() -> Result<Self, error::IndeterminateOffset> {
        let t = Self::now_utc();
        Ok(t.to_offset(UtcOffset::local_offset_at(t)?))
    }
    // endregion now

    /// Convert the `OffsetDateTime` from the current [`UtcOffset`] to the provided [`UtcOffset`].
    ///
    /// ```rust
    /// # use time::macros::{datetime, offset};
    /// assert_eq!(
    ///     datetime!("2000-01-01 0:00 UTC")
    ///         .to_offset(offset!("-1"))
    ///         .year(),
    ///     1999,
    /// );
    ///
    /// // Let's see what time Sydney's new year's celebration is in New York // and Los Angeles.
    ///
    /// // Construct midnight on new year's in Sydney.
    /// let sydney = datetime!("2000-01-01 0:00 +11");
    /// let new_york = sydney.to_offset(offset!("-5"));
    /// let los_angeles = sydney.to_offset(offset!("-8"));
    /// assert_eq!(sydney.hour(), 0);
    /// assert_eq!(new_york.hour(), 8);
    /// assert_eq!(los_angeles.hour(), 5);
    /// ```
    pub const fn to_offset(self, offset: UtcOffset) -> Self {
        Self {
            utc_datetime: self.utc_datetime,
            offset,
        }
    }

    // region: constructors
    /// Create an `OffsetDateTime` from the provided Unix timestamp. Calling `.offset()` on the
    /// resulting value is guaranteed to return UTC.
    ///
    /// ```rust
    /// # use time::{OffsetDateTime, macros::datetime};
    /// assert_eq!(
    ///     OffsetDateTime::from_unix_timestamp(0),
    ///     Ok(OffsetDateTime::UNIX_EPOCH),
    /// );
    /// assert_eq!(
    ///     OffsetDateTime::from_unix_timestamp(1_546_300_800),
    ///     Ok(datetime!("2019-01-01 0:00 UTC")),
    /// );
    /// ```
    ///
    /// If you have a timestamp-nanosecond pair, you can use something along the lines of the
    /// following:
    ///
    /// ```rust
    /// # use time::{Duration, OffsetDateTime, ext::NumericalDuration};
    /// let (timestamp, nanos) = (1, 500_000_000);
    /// assert_eq!(
    ///     OffsetDateTime::from_unix_timestamp(timestamp)? + Duration::nanoseconds(nanos),
    ///     OffsetDateTime::UNIX_EPOCH + 1.5.seconds()
    /// );
    /// # Ok::<_, time::Error>(())
    /// ```
    pub const fn from_unix_timestamp(timestamp: i64) -> Result<Self, error::ComponentRange> {
        #[allow(clippy::missing_docs_in_private_items)]
        const MIN_TIMESTAMP: i64 = Date::MIN.midnight().assume_utc().unix_timestamp();
        #[allow(clippy::missing_docs_in_private_items)]
        const MAX_TIMESTAMP: i64 = Date::MAX
            .with_time(Time::__from_hms_nanos_unchecked(23, 59, 59, 999_999_999))
            .assume_utc()
            .unix_timestamp();

        ensure_value_in_range!(timestamp in MIN_TIMESTAMP => MAX_TIMESTAMP);

        // Use the unchecked method here, as the input validity has already been verified.
        let date = Date::from_julian_day_unchecked(
            UNIX_EPOCH_JULIAN_DAY + div_floor!(timestamp, 86_400) as i32,
        );

        let seconds_within_day = rem_euclid!(timestamp, 86_400);
        let time = Time::__from_hms_nanos_unchecked(
            (seconds_within_day / 3_600) as _,
            ((seconds_within_day % 3_600) / 60) as _,
            (seconds_within_day % 60) as _,
            0,
        );

        Ok(PrimitiveDateTime::new(date, time).assume_utc())
    }

    /// Construct an `OffsetDateTime` from the provided Unix timestamp (in nanoseconds). Calling
    /// `.offset()` on the resulting value is guaranteed to return UTC.
    ///
    /// ```rust
    /// # use time::{OffsetDateTime, macros::datetime};
    /// assert_eq!(
    ///     OffsetDateTime::from_unix_timestamp_nanos(0),
    ///     Ok(OffsetDateTime::UNIX_EPOCH),
    /// );
    /// assert_eq!(
    ///     OffsetDateTime::from_unix_timestamp_nanos(1_546_300_800_000_000_000),
    ///     Ok(datetime!("2019-01-01 0:00 UTC")),
    /// );
    /// ```
    pub const fn from_unix_timestamp_nanos(timestamp: i128) -> Result<Self, error::ComponentRange> {
        let datetime = const_try!(Self::from_unix_timestamp(
            div_floor!(timestamp, 1_000_000_000) as i64
        ));

        Ok(datetime
            .utc_datetime
            .replace_time(Time::__from_hms_nanos_unchecked(
                datetime.utc_datetime.hour(),
                datetime.utc_datetime.minute(),
                datetime.utc_datetime.second(),
                rem_euclid!(timestamp, 1_000_000_000) as u32,
            ))
            .assume_utc())
    }
    // endregion constructors

    // region: getters
    /// Get the [`UtcOffset`].
    ///
    /// ```rust
    /// # use time::macros::{datetime, offset};
    /// assert_eq!(datetime!("2019-01-01 0:00 UTC").offset(), offset!("UTC"));
    /// assert_eq!(datetime!("2019-01-01 0:00 +1").offset(), offset!("+1"));
    /// ```
    pub const fn offset(self) -> UtcOffset {
        self.offset
    }

    /// Get the [Unix timestamp](https://en.wikipedia.org/wiki/Unix_time).
    ///
    /// ```rust
    /// # use time::macros::datetime;
    /// assert_eq!(datetime!("1970-01-01 0:00 UTC").unix_timestamp(), 0);
    /// assert_eq!(datetime!("1970-01-01 0:00 -1").unix_timestamp(), 3_600);
    /// ```
    pub const fn unix_timestamp(self) -> i64 {
        let days =
            (self.utc_datetime.to_julian_day() as i64 - UNIX_EPOCH_JULIAN_DAY as i64) * 86_400;
        let hours = self.utc_datetime.hour() as i64 * 3_600;
        let minutes = self.utc_datetime.minute() as i64 * 60;
        let seconds = self.utc_datetime.second() as i64;
        days + hours + minutes + seconds
    }

    /// Get the Unix timestamp in nanoseconds.
    ///
    /// ```rust
    /// use time::macros::datetime;
    /// assert_eq!(datetime!("1970-01-01 0:00 UTC").unix_timestamp_nanos(), 0);
    /// assert_eq!(
    ///     datetime!("1970-01-01 0:00 -1").unix_timestamp_nanos(),
    ///     3_600_000_000_000,
    /// );
    /// ```
    pub const fn unix_timestamp_nanos(self) -> i128 {
        self.unix_timestamp() as i128 * 1_000_000_000 + self.utc_datetime.nanosecond() as i128
    }

    /// Get the [`Date`] in the stored offset.
    ///
    /// ```rust
    /// # use time::macros::{date, datetime, offset};
    /// assert_eq!(datetime!("2019-01-01 0:00 UTC").date(), date!("2019-01-01"));
    /// assert_eq!(
    ///     datetime!("2019-01-01 0:00 UTC")
    ///         .to_offset(offset!("-1"))
    ///         .date(),
    ///     date!("2018-12-31"),
    /// );
    /// ```
    pub const fn date(self) -> Date {
        let second = self.utc_datetime.second() as i8 + self.offset.seconds_past_minute();
        let mut minute = self.utc_datetime.minute() as i8 + self.offset.minutes_past_hour();
        let mut hour = self.utc_datetime.hour() as i8 + self.offset.whole_hours();
        let (mut year, mut ordinal) = self.utc_datetime.date.to_ordinal_date();

        cascade!(!mut second in 0..60 => minute);
        cascade!(!mut minute in 0..60 => hour);
        cascade!(!mut hour in 0..24 => ordinal);
        cascade!(ordinal => year);

        Date::__from_ordinal_date_unchecked(year, ordinal)
    }

    /// Get the [`Time`] in the stored offset.
    ///
    /// ```rust
    /// # use time::macros::{datetime, offset, time};
    /// assert_eq!(datetime!("2019-01-01 0:00 UTC").time(), time!("0:00"));
    /// assert_eq!(
    ///     datetime!("2019-01-01 0:00 UTC")
    ///         .to_offset(offset!("-1"))
    ///         .time(),
    ///     time!("23:00")
    /// );
    /// ```
    pub const fn time(self) -> Time {
        let mut second = self.utc_datetime.second() as i8 + self.offset.seconds_past_minute();
        let mut minute = self.utc_datetime.minute() as i8 + self.offset.minutes_past_hour();
        let mut hour = self.utc_datetime.hour() as i8 + self.offset.whole_hours();

        cascade!(second in 0..60 => minute);
        cascade!(minute in 0..60 => hour);
        cascade!(hour in 0..24 => _);

        Time::__from_hms_nanos_unchecked(
            hour as _,
            minute as _,
            second as _,
            self.utc_datetime.nanosecond(),
        )
    }

    // region: date getters
    /// Get the year of the date in the stored offset.
    ///
    /// ```rust
    /// # use time::macros::{datetime, offset};
    /// assert_eq!(datetime!("2019-01-01 0:00 UTC").year(), 2019);
    /// assert_eq!(
    ///     datetime!("2019-12-31 23:00 UTC")
    ///         .to_offset(offset!("+1"))
    ///         .year(),
    ///     2020,
    /// );
    /// assert_eq!(datetime!("2020-01-01 0:00 UTC").year(), 2020);
    /// ```
    pub const fn year(self) -> i32 {
        let second = self.utc_datetime.second() as i8 + self.offset.seconds_past_minute();
        let mut minute = self.utc_datetime.minute() as i8 + self.offset.minutes_past_hour();
        let mut hour = self.utc_datetime.hour() as i8 + self.offset.whole_hours();
        let (mut year, mut ordinal) = self.utc_datetime.date.to_ordinal_date();

        cascade!(!mut second in 0..60 => minute);
        cascade!(!mut minute in 0..60 => hour);
        cascade!(!mut hour in 0..24 => ordinal);
        cascade!(!mut ordinal => year);

        year
    }

    /// Get the month of the date in the stored offset.
    ///
    /// The returned value will always be in the range `1..=12`.
    ///
    /// ```rust
    /// # use time::macros::{datetime, offset};
    /// assert_eq!(datetime!("2019-01-01 0:00 UTC").month(), 1);
    /// assert_eq!(
    ///     datetime!("2019-12-31 23:00 UTC")
    ///         .to_offset(offset!("+1"))
    ///         .month(),
    ///     1,
    /// );
    /// ```
    pub const fn month(self) -> u8 {
        self.date().month()
    }

    /// Get the day of the date in the stored offset.
    ///
    /// The returned value will always be in the range `1..=31`.
    ///
    /// ```rust
    /// # use time::macros::{datetime, offset};
    /// assert_eq!(datetime!("2019-01-01 0:00 UTC").day(), 1);
    /// assert_eq!(
    ///     datetime!("2019-12-31 23:00 UTC")
    ///         .to_offset(offset!("+1"))
    ///         .day(),
    ///     1,
    /// );
    /// ```
    pub const fn day(self) -> u8 {
        self.date().day()
    }

    /// Get the day of the year of the date in the stored offset.
    ///
    /// The returned value will always be in the range `1..=366`.
    ///
    /// ```rust
    /// # use time::macros::{datetime, offset};
    /// assert_eq!(datetime!("2019-01-01 0:00 UTC").ordinal(), 1);
    /// assert_eq!(
    ///     datetime!("2019-12-31 23:00 UTC")
    ///         .to_offset(offset!("+1"))
    ///         .ordinal(),
    ///     1,
    /// );
    /// ```
    pub const fn ordinal(self) -> u16 {
        let second = self.utc_datetime.second() as i8 + self.offset.seconds_past_minute();
        let mut minute = self.utc_datetime.minute() as i8 + self.offset.minutes_past_hour();
        let mut hour = self.utc_datetime.hour() as i8 + self.offset.whole_hours();
        let (year, mut ordinal) = self.utc_datetime.date.to_ordinal_date();

        cascade!(!mut second in 0..60 => minute);
        cascade!(!mut minute in 0..60 => hour);
        cascade!(!mut hour in 0..24 => ordinal);
        cascade!(ordinal => !mut year);

        ordinal
    }

    /// Get the ISO week number of the date in the stored offset.
    ///
    /// The returned value will always be in the range `1..=53`.
    ///
    /// ```rust
    /// # use time::macros::datetime;
    /// assert_eq!(datetime!("2019-01-01 0:00 UTC").iso_week(), 1);
    /// assert_eq!(datetime!("2020-01-01 0:00 UTC").iso_week(), 1);
    /// assert_eq!(datetime!("2020-12-31 0:00 UTC").iso_week(), 53);
    /// assert_eq!(datetime!("2021-01-01 0:00 UTC").iso_week(), 53);
    /// ```
    pub const fn iso_week(self) -> u8 {
        self.date().iso_week()
    }

    /// Get the week number where week 1 begins on the first Sunday.
    ///
    /// The returned value will always be in the range `0..=53`.
    ///
    /// ```rust
    /// # use time::macros::datetime;
    /// assert_eq!(datetime!("2019-01-01 0:00 UTC").sunday_based_week(), 0);
    /// assert_eq!(datetime!("2020-01-01 0:00 UTC").sunday_based_week(), 0);
    /// assert_eq!(datetime!("2020-12-31 0:00 UTC").sunday_based_week(), 52);
    /// assert_eq!(datetime!("2021-01-01 0:00 UTC").sunday_based_week(), 0);
    /// ```
    pub const fn sunday_based_week(self) -> u8 {
        self.date().sunday_based_week()
    }

    /// Get the week number where week 1 begins on the first Monday.
    ///
    /// The returned value will always be in the range `0..=53`.
    ///
    /// ```rust
    /// # use time::macros::datetime;
    /// assert_eq!(datetime!("2019-01-01 0:00 UTC").monday_based_week(), 0);
    /// assert_eq!(datetime!("2020-01-01 0:00 UTC").monday_based_week(), 0);
    /// assert_eq!(datetime!("2020-12-31 0:00 UTC").monday_based_week(), 52);
    /// assert_eq!(datetime!("2021-01-01 0:00 UTC").monday_based_week(), 0);
    /// ```
    pub const fn monday_based_week(self) -> u8 {
        self.date().monday_based_week()
    }

    /// Get the year, month, and day.
    ///
    /// ```rust
    /// # use time::macros::datetime;
    /// assert_eq!(
    ///     datetime!("2019-01-01 0:00 UTC").to_calendar_date(),
    ///     (2019, 1, 1)
    /// );
    /// ```
    pub const fn to_calendar_date(self) -> (i32, u8, u8) {
        self.date().to_calendar_date()
    }

    /// Get the year and ordinal day number.
    ///
    /// ```rust
    /// # use time::macros::datetime;
    /// assert_eq!(
    ///     datetime!("2019-01-01 0:00 UTC").to_ordinal_date(),
    ///     (2019, 1)
    /// );
    /// ```
    pub const fn to_ordinal_date(self) -> (i32, u16) {
        self.date().to_ordinal_date()
    }

    /// Get the ISO 8601 year, week number, and weekday.
    ///
    /// ```rust
    /// # use time::{Weekday::*, macros::datetime};
    /// assert_eq!(
    ///     datetime!("2019-01-01 0:00 UTC").to_iso_week_date(),
    ///     (2019, 1, Tuesday)
    /// );
    /// assert_eq!(
    ///     datetime!("2019-10-04 0:00 UTC").to_iso_week_date(),
    ///     (2019, 40, Friday)
    /// );
    /// assert_eq!(
    ///     datetime!("2020-01-01 0:00 UTC").to_iso_week_date(),
    ///     (2020, 1, Wednesday)
    /// );
    /// assert_eq!(
    ///     datetime!("2020-12-31 0:00 UTC").to_iso_week_date(),
    ///     (2020, 53, Thursday)
    /// );
    /// assert_eq!(
    ///     datetime!("2021-01-01 0:00 UTC").to_iso_week_date(),
    ///     (2020, 53, Friday)
    /// );
    /// ```
    pub const fn to_iso_week_date(self) -> (i32, u8, Weekday) {
        self.date().to_iso_week_date()
    }

    /// Get the weekday of the date in the stored offset.
    ///
    /// ```rust
    /// # use time::{Weekday::*, macros::datetime};
    /// assert_eq!(datetime!("2019-01-01 0:00 UTC").weekday(), Tuesday);
    /// assert_eq!(datetime!("2019-02-01 0:00 UTC").weekday(), Friday);
    /// assert_eq!(datetime!("2019-03-01 0:00 UTC").weekday(), Friday);
    /// ```
    pub const fn weekday(self) -> Weekday {
        self.date().weekday()
    }

    /// Get the Julian day for the date. The time is not taken into account for this calculation.
    ///
    /// The algorithm to perform this conversion is derived from one provided by Peter Baum; it is
    /// freely available [here](https://www.researchgate.net/publication/316558298_Date_Algorithms).
    ///
    /// ```rust
    /// # use time::macros::datetime;
    /// assert_eq!(datetime!("-4713-11-24 0:00 UTC").to_julian_day(), 0);
    /// assert_eq!(datetime!("2000-01-01 0:00 UTC").to_julian_day(), 2_451_545);
    /// assert_eq!(datetime!("2019-01-01 0:00 UTC").to_julian_day(), 2_458_485);
    /// assert_eq!(datetime!("2019-12-31 0:00 UTC").to_julian_day(), 2_458_849);
    /// ```
    pub const fn to_julian_day(self) -> i32 {
        self.date().to_julian_day()
    }
    // endregion date getters

    // region: time getters
    /// Get the clock hour, minute, and second.
    ///
    /// ```rust
    /// # use time::macros::datetime;
    /// assert_eq!(datetime!("2020-01-01 0:00:00 UTC").to_hms(), (0, 0, 0));
    /// assert_eq!(datetime!("2020-01-01 23:59:59 UTC").to_hms(), (23, 59, 59));
    /// ```
    pub const fn to_hms(self) -> (u8, u8, u8) {
        self.time().as_hms()
    }

    /// Get the clock hour, minute, second, and millisecond.
    ///
    /// ```rust
    /// # use time::macros::datetime;
    /// assert_eq!(
    ///     datetime!("2020-01-01 0:00:00 UTC").to_hms_milli(),
    ///     (0, 0, 0, 0)
    /// );
    /// assert_eq!(
    ///     datetime!("2020-01-01 23:59:59.999 UTC").to_hms_milli(),
    ///     (23, 59, 59, 999)
    /// );
    /// ```
    pub const fn to_hms_milli(self) -> (u8, u8, u8, u16) {
        self.time().as_hms_milli()
    }

    /// Get the clock hour, minute, second, and microsecond.
    ///
    /// ```rust
    /// # use time::macros::datetime;
    /// assert_eq!(
    ///     datetime!("2020-01-01 0:00:00 UTC").to_hms_micro(),
    ///     (0, 0, 0, 0)
    /// );
    /// assert_eq!(
    ///     datetime!("2020-01-01 23:59:59.999_999 UTC").to_hms_micro(),
    ///     (23, 59, 59, 999_999)
    /// );
    /// ```
    pub const fn to_hms_micro(self) -> (u8, u8, u8, u32) {
        self.time().as_hms_micro()
    }

    /// Get the clock hour, minute, second, and nanosecond.
    ///
    /// ```rust
    /// # use time::macros::datetime;
    /// assert_eq!(
    ///     datetime!("2020-01-01 0:00:00 UTC").to_hms_nano(),
    ///     (0, 0, 0, 0)
    /// );
    /// assert_eq!(
    ///     datetime!("2020-01-01 23:59:59.999_999_999 UTC").to_hms_nano(),
    ///     (23, 59, 59, 999_999_999)
    /// );
    /// ```
    pub const fn to_hms_nano(self) -> (u8, u8, u8, u32) {
        self.time().as_hms_nano()
    }

    /// Get the clock hour in the stored offset.
    ///
    /// The returned value will always be in the range `0..24`.
    ///
    /// ```rust
    /// # use time::macros::{datetime, offset};
    /// assert_eq!(datetime!("2019-01-01 0:00 UTC").hour(), 0);
    /// assert_eq!(
    ///     datetime!("2019-01-01 23:59:59 UTC")
    ///         .to_offset(offset!("-2"))
    ///         .hour(),
    ///     21,
    /// );
    /// ```
    pub const fn hour(self) -> u8 {
        let second = self.utc_datetime.second() as i8 + self.offset.seconds_past_minute();
        let mut minute = self.utc_datetime.minute() as i8 + self.offset.minutes_past_hour();
        let mut hour = self.utc_datetime.hour() as i8 + self.offset.whole_hours();

        cascade!(!mut second in 0..60 => minute);
        cascade!(!mut minute in 0..60 => hour);
        cascade!(hour in 0..24 => _);

        hour as _
    }

    /// Get the minute within the hour in the stored offset.
    ///
    /// The returned value will always be in the range `0..60`.
    ///
    /// ```rust
    /// # use time::macros::{datetime, offset};
    /// assert_eq!(datetime!("2019-01-01 0:00 UTC").minute(), 0);
    /// assert_eq!(
    ///     datetime!("2019-01-01 23:59:59 UTC")
    ///         .to_offset(offset!("+0:30"))
    ///         .minute(),
    ///     29,
    /// );
    /// ```
    pub const fn minute(self) -> u8 {
        let second = self.utc_datetime.second() as i8 + self.offset.seconds_past_minute();
        let mut minute = self.utc_datetime.minute() as i8 + self.offset.minutes_past_hour();

        cascade!(!mut second in 0..60 => minute);
        cascade!(minute in 0..60 => _);

        minute as _
    }

    /// Get the second within the minute in the stored offset.
    ///
    /// The returned value will always be in the range `0..60`.
    ///
    /// ```rust
    /// # use time::macros::{datetime, offset};
    /// assert_eq!(datetime!("2019-01-01 0:00 UTC").second(), 0);
    /// assert_eq!(
    ///     datetime!("2019-01-01 23:59:59 UTC")
    ///         .to_offset(offset!("+0:00:30"))
    ///         .second(),
    ///     29,
    /// );
    /// ```
    pub const fn second(self) -> u8 {
        let mut second = self.utc_datetime.second() as i8 + self.offset.seconds_past_minute();
        cascade!(second in 0..60 => _);
        second as _
    }

    // Because a `UtcOffset` is limited in resolution to one second, any subsecond value will not
    // change when adjusting for the offset.

    /// Get the milliseconds within the second in the stored offset.
    ///
    /// The returned value will always be in the range `0..1_000`.
    ///
    /// ```rust
    /// # use time::macros::datetime;
    /// assert_eq!(datetime!("2019-01-01 0:00 UTC").millisecond(), 0);
    /// assert_eq!(datetime!("2019-01-01 23:59:59.999 UTC").millisecond(), 999);
    /// ```
    pub const fn millisecond(self) -> u16 {
        self.utc_datetime.millisecond()
    }

    /// Get the microseconds within the second in the stored offset.
    ///
    /// The returned value will always be in the range `0..1_000_000`.
    ///
    /// ```rust
    /// # use time::macros::datetime;
    /// assert_eq!(datetime!("2019-01-01 0:00 UTC").microsecond(), 0);
    /// assert_eq!(
    ///     datetime!("2019-01-01 23:59:59.999_999 UTC").microsecond(),
    ///     999_999,
    /// );
    /// ```
    pub const fn microsecond(self) -> u32 {
        self.utc_datetime.microsecond()
    }

    /// Get the nanoseconds within the second in the stored offset.
    ///
    /// The returned value will always be in the range `0..1_000_000_000`.
    ///
    /// ```rust
    /// # use time::macros::datetime;
    /// assert_eq!(datetime!("2019-01-01 0:00 UTC").nanosecond(), 0);
    /// assert_eq!(
    ///     datetime!("2019-01-01 23:59:59.999_999_999 UTC").nanosecond(),
    ///     999_999_999,
    /// );
    /// ```
    pub const fn nanosecond(self) -> u32 {
        self.utc_datetime.nanosecond()
    }
    // endregion time getters
    // endregion getters
}

// region: replacement
/// Methods that replace part of the `OffsetDateTime`.
impl OffsetDateTime {
    /// Replace the time, which is assumed to be in the stored offset. The date and offset
    /// components are unchanged.
    ///
    /// ```rust
    /// # use time::macros::{datetime, time};
    /// assert_eq!(
    ///     datetime!("2020-01-01 5:00 UTC").replace_time(time!("12:00")),
    ///     datetime!("2020-01-01 12:00 UTC")
    /// );
    /// assert_eq!(
    ///     datetime!("2020-01-01 12:00 -5").replace_time(time!("7:00")),
    ///     datetime!("2020-01-01 7:00 -5")
    /// );
    /// assert_eq!(
    ///     datetime!("2020-01-01 0:00 +1").replace_time(time!("12:00")),
    ///     datetime!("2020-01-01 12:00 +1")
    /// );
    /// ```
    #[must_use = "This method does not mutate the original `OffsetDateTime`."]
    pub const fn replace_time(self, time: Time) -> Self {
        self.utc_datetime
            .utc_to_offset(self.offset)
            .replace_time(time)
            .assume_offset(self.offset)
    }

    /// Replace the date, which is assumed to be in the stored offset. The time and offset
    /// components are unchanged.
    ///
    /// ```rust
    /// # use time::macros::{datetime, date};
    /// assert_eq!(
    ///     datetime!("2020-01-01 12:00 UTC").replace_date(date!("2020-01-30")),
    ///     datetime!("2020-01-30 12:00 UTC")
    /// );
    /// assert_eq!(
    ///     datetime!("2020-01-01 0:00 +1").replace_date(date!("2020-01-30")),
    ///     datetime!("2020-01-30 0:00 +1")
    /// );
    /// ```
    #[must_use = "This method does not mutate the original `OffsetDateTime`."]
    pub const fn replace_date(self, date: Date) -> Self {
        self.utc_datetime
            .utc_to_offset(self.offset)
            .replace_date(date)
            .assume_offset(self.offset)
    }

    /// Replace the date and time, which are assumed to be in the stored offset. The offset
    /// component remains unchanged.
    ///
    /// ```rust
    /// # use time::macros::datetime;
    /// assert_eq!(
    ///     datetime!("2020-01-01 12:00 UTC").replace_date_time(datetime!("2020-01-30 16:00")),
    ///     datetime!("2020-01-30 16:00 UTC")
    /// );
    /// assert_eq!(
    ///     datetime!("2020-01-01 12:00 +1").replace_date_time(datetime!("2020-01-30 0:00")),
    ///     datetime!("2020-01-30 0:00 +1")
    /// );
    /// ```
    #[must_use = "This method does not mutate the original `OffsetDateTime`."]
    pub const fn replace_date_time(self, date_time: PrimitiveDateTime) -> Self {
        date_time.assume_offset(self.offset)
    }

    /// Replace the offset. The date and time components remain unchanged.
    ///
    /// ```rust
    /// # use time::macros::{datetime, offset};
    /// assert_eq!(
    ///     datetime!("2020-01-01 0:00 UTC").replace_offset(offset!("-5")),
    ///     datetime!("2020-01-01 0:00 -5")
    /// );
    /// ```
    #[must_use = "This method does not mutate the original `OffsetDateTime`."]
    pub const fn replace_offset(self, offset: UtcOffset) -> Self {
        self.utc_datetime.assume_offset(offset)
    }
}
// endregion replacement

// region: formatting & parsing
#[cfg(feature = "formatting")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "formatting")))]
impl OffsetDateTime {
    /// Format the `OffsetDateTime` using the provided format description. The formatted value will
    /// be output to the provided writer. The format description will typically be parsed by using
    /// [`format_description::parse`](crate::format_description::parse()).
    pub fn format_into<F: Formattable>(
        self,
        output: &mut impl io::Write,
        format: &F,
    ) -> Result<usize, F::Error> {
        let local = self.utc_datetime.utc_to_offset(self.offset);
        format.format_into(
            output,
            Some(local.date),
            Some(local.time),
            Some(self.offset),
        )
    }

    /// Format the `OffsetDateTime` using the provided format description. The format description
    /// will typically be parsed by using
    /// [`format_description::parse`](crate::format_description::parse()).
    ///
    /// ```rust
    /// # use time::{format_description, macros::datetime};
    /// let format = format_description::parse(
    ///     "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour \
    ///          sign:mandatory]:[offset_minute]:[offset_second]",
    /// )?;
    /// assert_eq!(
    ///     datetime!("2020-01-02 03:04:05 +06:07:08").format(&format)?,
    ///     "2020-01-02 03:04:05 +06:07:08"
    /// );
    /// # Ok::<_, time::Error>(())
    /// ```
    pub fn format<F: Formattable>(self, format: &F) -> Result<String, F::Error> {
        let local = self.utc_datetime.utc_to_offset(self.offset);
        format.format(Some(local.date), Some(local.time), Some(self.offset))
    }
}

#[cfg(feature = "parsing")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "parsing")))]
impl OffsetDateTime {
    /// Parse a `PrimitiveDateTime` from the input using the provided format description. The format
    /// description will typically be parsed by using
    /// [`format_description::parse`](crate::format_description::parse()).
    ///
    /// ```rust
    /// # use time::{format_description, macros::datetime, OffsetDateTime};
    /// let format = format_description::parse(
    ///     "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour \
    ///          sign:mandatory]:[offset_minute]:[offset_second]",
    /// )?;
    /// assert_eq!(
    ///     OffsetDateTime::parse("2020-01-02 03:04:05 +06:07:08", &format)?,
    ///     datetime!("2020-01-02 03:04:05 +06:07:08")
    /// );
    /// # Ok::<_, time::Error>(())
    /// ```
    pub fn parse(input: &str, description: &impl Parsable) -> Result<Self, error::Parse> {
        description.parse_offset_date_time(input.as_bytes())
    }
}

#[cfg(feature = "formatting")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "formatting")))]
impl fmt::Display for OffsetDateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.date(), self.time(), self.offset)
    }
}
// endregion formatting & parsing

// region: trait impls
impl PartialEq for OffsetDateTime {
    fn eq(&self, rhs: &Self) -> bool {
        self.utc_datetime.eq(&rhs.utc_datetime)
    }
}

impl PartialOrd for OffsetDateTime {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}

impl Ord for OffsetDateTime {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.utc_datetime.cmp(&rhs.utc_datetime)
    }
}

impl Hash for OffsetDateTime {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        // We need to distinguish this from a `PrimitiveDateTime`, which would otherwise conflict.
        hasher.write(b"OffsetDateTime");
        self.utc_datetime.hash(hasher);
    }
}

impl Add<Duration> for OffsetDateTime {
    type Output = Self;

    fn add(self, duration: Duration) -> Self::Output {
        Self {
            utc_datetime: self.utc_datetime + duration,
            offset: self.offset,
        }
    }
}

impl Add<StdDuration> for OffsetDateTime {
    type Output = Self;

    fn add(self, duration: StdDuration) -> Self::Output {
        Self {
            utc_datetime: self.utc_datetime + duration,
            offset: self.offset,
        }
    }
}

impl AddAssign<Duration> for OffsetDateTime {
    fn add_assign(&mut self, duration: Duration) {
        *self = *self + duration;
    }
}

impl AddAssign<StdDuration> for OffsetDateTime {
    fn add_assign(&mut self, duration: StdDuration) {
        *self = *self + duration;
    }
}

impl Sub<Duration> for OffsetDateTime {
    type Output = Self;

    fn sub(self, duration: Duration) -> Self::Output {
        Self {
            utc_datetime: self.utc_datetime - duration,
            offset: self.offset,
        }
    }
}

impl Sub<StdDuration> for OffsetDateTime {
    type Output = Self;

    fn sub(self, duration: StdDuration) -> Self::Output {
        Self {
            utc_datetime: self.utc_datetime - duration,
            offset: self.offset,
        }
    }
}

impl SubAssign<Duration> for OffsetDateTime {
    fn sub_assign(&mut self, duration: Duration) {
        *self = *self - duration;
    }
}

impl SubAssign<StdDuration> for OffsetDateTime {
    fn sub_assign(&mut self, duration: StdDuration) {
        *self = *self - duration;
    }
}

impl Sub<OffsetDateTime> for OffsetDateTime {
    type Output = Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        self.utc_datetime - rhs.utc_datetime
    }
}

#[cfg(feature = "std")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "std")))]
impl Add<Duration> for SystemTime {
    type Output = Self;

    fn add(self, duration: Duration) -> Self::Output {
        if duration.is_zero() {
            self
        } else if duration.is_positive() {
            self + duration.abs_std()
        } else {
            // duration.is_negative()
            self - duration.abs_std()
        }
    }
}

#[cfg(feature = "std")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "std")))]
impl AddAssign<Duration> for SystemTime {
    fn add_assign(&mut self, duration: Duration) {
        *self = *self + duration;
    }
}

#[cfg(feature = "std")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "std")))]
impl Sub<Duration> for SystemTime {
    type Output = Self;

    fn sub(self, duration: Duration) -> Self::Output {
        (OffsetDateTime::from(self) - duration).into()
    }
}

#[cfg(feature = "std")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "std")))]
impl SubAssign<Duration> for SystemTime {
    fn sub_assign(&mut self, duration: Duration) {
        *self = *self - duration;
    }
}

#[cfg(feature = "std")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "std")))]
impl Sub<SystemTime> for OffsetDateTime {
    type Output = Duration;

    fn sub(self, rhs: SystemTime) -> Self::Output {
        self - Self::from(rhs)
    }
}

#[cfg(feature = "std")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "std")))]
impl Sub<OffsetDateTime> for SystemTime {
    type Output = Duration;

    fn sub(self, rhs: OffsetDateTime) -> Self::Output {
        OffsetDateTime::from(self) - rhs
    }
}

#[cfg(feature = "std")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "std")))]
impl PartialEq<SystemTime> for OffsetDateTime {
    fn eq(&self, rhs: &SystemTime) -> bool {
        self == &Self::from(*rhs)
    }
}

#[cfg(feature = "std")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "std")))]
impl PartialEq<OffsetDateTime> for SystemTime {
    fn eq(&self, rhs: &OffsetDateTime) -> bool {
        &OffsetDateTime::from(*self) == rhs
    }
}

#[cfg(feature = "std")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "std")))]
impl PartialOrd<SystemTime> for OffsetDateTime {
    fn partial_cmp(&self, other: &SystemTime) -> Option<Ordering> {
        self.partial_cmp(&Self::from(*other))
    }
}

#[cfg(feature = "std")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "std")))]
impl PartialOrd<OffsetDateTime> for SystemTime {
    fn partial_cmp(&self, other: &OffsetDateTime) -> Option<Ordering> {
        OffsetDateTime::from(*self).partial_cmp(other)
    }
}

#[cfg(feature = "std")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "std")))]
impl From<SystemTime> for OffsetDateTime {
    fn from(system_time: SystemTime) -> Self {
        match system_time.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(duration) => Self::UNIX_EPOCH + duration,
            Err(err) => Self::UNIX_EPOCH - err.duration(),
        }
    }
}

#[cfg(feature = "std")]
#[cfg_attr(__time_03_docs, doc(cfg(feature = "std")))]
impl From<OffsetDateTime> for SystemTime {
    fn from(datetime: OffsetDateTime) -> Self {
        let duration = datetime - OffsetDateTime::UNIX_EPOCH;

        if duration.is_zero() {
            Self::UNIX_EPOCH
        } else if duration.is_positive() {
            Self::UNIX_EPOCH + duration.abs_std()
        } else {
            // duration.is_negative()
            Self::UNIX_EPOCH - duration.abs_std()
        }
    }
}
// endregion trait impls
