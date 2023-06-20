//! Information parsed from an input and format description.

use core::convert::{TryFrom, TryInto};
use core::num::{NonZeroU16, NonZeroU8};

use crate::error::TryFromParsed::InsufficientInformation;
use crate::format_description::modifier::{WeekNumberRepr, YearRepr};
use crate::format_description::{Component, FormatItem};
use crate::parsing::component::{
    parse_day, parse_hour, parse_minute, parse_month, parse_offset_hour, parse_offset_minute,
    parse_offset_second, parse_ordinal, parse_period, parse_second, parse_subsecond,
    parse_week_number, parse_weekday, parse_year, Period,
};
use crate::parsing::ParsedItem;
use crate::{error, Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset, Weekday};

/// All information parsed.
///
/// This information is directly used to construct the final values.
///
/// Most users will not need think about this struct in any way. It is public to allow for manual
/// control over values, in the instance that the default parser is insufficient.
#[derive(Debug, Clone, Copy)]
pub struct Parsed {
    /// Calendar year.
    pub(crate) year: Option<i32>,
    /// The last two digits of the calendar year.
    pub(crate) year_last_two: Option<u8>,
    /// Year of the [ISO week date](https://en.wikipedia.org/wiki/ISO_week_date).
    pub(crate) iso_year: Option<i32>,
    /// The last two digits of the ISO week year.
    pub(crate) iso_year_last_two: Option<u8>,
    /// Month of the year.
    pub(crate) month: Option<Month>,
    /// Week of the year, where week one begins on the first Sunday of the calendar year.
    pub(crate) sunday_week_number: Option<u8>,
    /// Week of the year, where week one begins on the first Monday of the calendar year.
    pub(crate) monday_week_number: Option<u8>,
    /// Week of the year, where week one is the Monday-to-Sunday period containing January 4.
    pub(crate) iso_week_number: Option<NonZeroU8>,
    /// Day of the week.
    pub(crate) weekday: Option<Weekday>,
    /// Day of the year.
    pub(crate) ordinal: Option<NonZeroU16>,
    /// Day of the month.
    pub(crate) day: Option<NonZeroU8>,
    /// Hour within the day.
    pub(crate) hour_24: Option<u8>,
    /// Hour within the 12-hour period (midnight to noon or vice versa). This is typically used in
    /// conjunction with AM/PM, which is indicated by the `hour_12_is_pm` field.
    pub(crate) hour_12: Option<NonZeroU8>,
    /// Whether the `hour_12` field indicates a time that "PM".
    pub(crate) hour_12_is_pm: Option<bool>,
    /// Minute within the hour.
    pub(crate) minute: Option<u8>,
    /// Second within the minute.
    pub(crate) second: Option<u8>,
    /// Nanosecond within the second.
    pub(crate) subsecond: Option<u32>,
    /// Whole hours of the UTC offset.
    pub(crate) offset_hour: Option<i8>,
    /// Minutes within the hour of the UTC offset.
    pub(crate) offset_minute: Option<u8>,
    /// Seconds within the minute of the UTC offset.
    pub(crate) offset_second: Option<u8>,
}

impl Parsed {
    /// Create a new instance of `Parsed` with no information known.
    pub const fn new() -> Self {
        Self {
            year: None,
            year_last_two: None,
            iso_year: None,
            iso_year_last_two: None,
            month: None,
            sunday_week_number: None,
            monday_week_number: None,
            iso_week_number: None,
            weekday: None,
            ordinal: None,
            day: None,
            hour_24: None,
            hour_12: None,
            hour_12_is_pm: None,
            minute: None,
            second: None,
            subsecond: None,
            offset_hour: None,
            offset_minute: None,
            offset_second: None,
        }
    }

    /// Parse a single [`FormatItem`], mutating the struct. The remaining input is returned as the
    /// `Ok` value.
    ///
    /// If a [`FormatItem::Optional`] is passed, parsing will not fail; the input will be returned
    /// as-is if the expected format is not present.
    pub fn parse_item<'a>(
        &mut self,
        input: &'a [u8],
        item: &FormatItem<'_>,
    ) -> Result<&'a [u8], error::ParseFromDescription> {
        match item {
            FormatItem::Literal(literal) => Self::parse_literal(input, literal),
            FormatItem::Component(component) => self.parse_component(input, *component),
            FormatItem::Compound(compound) => self.parse_items(input, compound),
            FormatItem::Optional(item) => self.parse_item(input, item).or(Ok(input)),
            FormatItem::First(items) => {
                let mut first_err = None;

                for item in items.iter() {
                    match self.parse_item(input, item) {
                        Ok(remaining_input) => return Ok(remaining_input),
                        Err(err) if first_err.is_none() => first_err = Some(err),
                        Err(_) => {}
                    }
                }

                match first_err {
                    Some(err) => Err(err),
                    // This location will be reached if the slice is empty, skipping the `for` loop.
                    // As this case is expected to be uncommon, there's no need to check up front.
                    None => Ok(input),
                }
            }
        }
    }

    /// Parse a sequence of [`FormatItem`]s, mutating the struct. The remaining input is returned as
    /// the `Ok` value.
    ///
    /// This method will fail if any of the contained [`FormatItem`]s fail to parse. `self` will not
    /// be mutated in this instance.
    pub fn parse_items<'a>(
        &mut self,
        mut input: &'a [u8],
        items: &[FormatItem<'_>],
    ) -> Result<&'a [u8], error::ParseFromDescription> {
        // Make a copy that we can mutate. It will only be set to the user's copy if everything
        // succeeds.
        let mut this = *self;
        for item in items {
            input = this.parse_item(input, item)?;
        }
        *self = this;
        Ok(input)
    }

    /// Parse a literal byte sequence. The remaining input is returned as the `Ok` value.
    pub fn parse_literal<'a>(
        input: &'a [u8],
        literal: &[u8],
    ) -> Result<&'a [u8], error::ParseFromDescription> {
        input
            .strip_prefix(literal)
            .ok_or(error::ParseFromDescription::InvalidLiteral)
    }

    /// Parse a single component, mutating the struct. The remaining input is returned as the `Ok`
    /// value.
    pub fn parse_component<'a>(
        &mut self,
        input: &'a [u8],
        component: Component,
    ) -> Result<&'a [u8], error::ParseFromDescription> {
        use error::ParseFromDescription::InvalidComponent;

        match component {
            Component::Day(modifiers) => Ok(parse_day(input, modifiers)
                .ok_or(InvalidComponent("day"))?
                .assign_value_to(&mut self.day)),
            Component::Month(modifiers) => Ok(parse_month(input, modifiers)
                .ok_or(InvalidComponent("month"))?
                .assign_value_to(&mut self.month)),
            Component::Ordinal(modifiers) => Ok(parse_ordinal(input, modifiers)
                .ok_or(InvalidComponent("ordinal"))?
                .assign_value_to(&mut self.ordinal)),
            Component::Weekday(modifiers) => Ok(parse_weekday(input, modifiers)
                .ok_or(InvalidComponent("weekday"))?
                .assign_value_to(&mut self.weekday)),
            Component::WeekNumber(modifiers) => {
                let ParsedItem(remaining, value) =
                    parse_week_number(input, modifiers).ok_or(InvalidComponent("week number"))?;
                match modifiers.repr {
                    WeekNumberRepr::Iso => {
                        self.iso_week_number =
                            Some(NonZeroU8::new(value).ok_or(InvalidComponent("week number"))?);
                    }
                    WeekNumberRepr::Sunday => self.sunday_week_number = Some(value),
                    WeekNumberRepr::Monday => self.monday_week_number = Some(value),
                }
                Ok(remaining)
            }
            Component::Year(modifiers) => {
                let ParsedItem(remaining, value) =
                    parse_year(input, modifiers).ok_or(InvalidComponent("year"))?;
                match (modifiers.iso_week_based, modifiers.repr) {
                    (false, YearRepr::Full) => self.year = Some(value),
                    (false, YearRepr::LastTwo) => self.year_last_two = Some(value as u8),
                    (true, YearRepr::Full) => self.iso_year = Some(value),
                    (true, YearRepr::LastTwo) => self.iso_year_last_two = Some(value as u8),
                }
                Ok(remaining)
            }
            Component::Hour(modifiers) => {
                let ParsedItem(remaining, value) =
                    parse_hour(input, modifiers).ok_or(InvalidComponent("hour"))?;
                if modifiers.is_12_hour_clock {
                    self.hour_12 = Some(NonZeroU8::new(value).ok_or(InvalidComponent("hour"))?);
                } else {
                    self.hour_24 = Some(value);
                }
                Ok(remaining)
            }
            Component::Minute(modifiers) => Ok(parse_minute(input, modifiers)
                .ok_or(InvalidComponent("minute"))?
                .assign_value_to(&mut self.minute)),
            Component::Period(modifiers) => Ok(parse_period(input, modifiers)
                .ok_or(InvalidComponent("period"))?
                .map(|period| period == Period::Pm)
                .assign_value_to(&mut self.hour_12_is_pm)),
            Component::Second(modifiers) => Ok(parse_second(input, modifiers)
                .ok_or(InvalidComponent("second"))?
                .assign_value_to(&mut self.second)),
            Component::Subsecond(modifiers) => Ok(parse_subsecond(input, modifiers)
                .ok_or(InvalidComponent("subsecond"))?
                .assign_value_to(&mut self.subsecond)),
            Component::OffsetHour(modifiers) => Ok(parse_offset_hour(input, modifiers)
                .ok_or(InvalidComponent("offset hour"))?
                .assign_value_to(&mut self.offset_hour)),
            Component::OffsetMinute(modifiers) => Ok(parse_offset_minute(input, modifiers)
                .ok_or(InvalidComponent("offset minute"))?
                .assign_value_to(&mut self.offset_minute)),
            Component::OffsetSecond(modifiers) => Ok(parse_offset_second(input, modifiers)
                .ok_or(InvalidComponent("offset second"))?
                .assign_value_to(&mut self.offset_second)),
        }
    }
}

/// Getter methods
impl Parsed {
    /// Obtain the `year` component.
    pub const fn year(&self) -> Option<i32> {
        self.year
    }

    /// Obtain the `year_last_two` component.
    pub const fn year_last_two(&self) -> Option<u8> {
        self.year_last_two
    }

    /// Obtain the `iso_year` component.
    pub const fn iso_year(&self) -> Option<i32> {
        self.iso_year
    }

    /// Obtain the `iso_year_last_two` component.
    pub const fn iso_year_last_two(&self) -> Option<u8> {
        self.iso_year_last_two
    }

    /// Obtain the `month` component.
    pub const fn month(&self) -> Option<Month> {
        self.month
    }

    /// Obtain the `sunday_week_number` component.
    pub const fn sunday_week_number(&self) -> Option<u8> {
        self.sunday_week_number
    }

    /// Obtain the `monday_week_number` component.
    pub const fn monday_week_number(&self) -> Option<u8> {
        self.monday_week_number
    }

    /// Obtain the `iso_week_number` component.
    pub const fn iso_week_number(&self) -> Option<NonZeroU8> {
        self.iso_week_number
    }

    /// Obtain the `weekday` component.
    pub const fn weekday(&self) -> Option<Weekday> {
        self.weekday
    }

    /// Obtain the `ordinal` component.
    pub const fn ordinal(&self) -> Option<NonZeroU16> {
        self.ordinal
    }

    /// Obtain the `day` component.
    pub const fn day(&self) -> Option<NonZeroU8> {
        self.day
    }

    /// Obtain the `hour_24` component.
    pub const fn hour_24(&self) -> Option<u8> {
        self.hour_24
    }

    /// Obtain the `hour_12` component.
    pub const fn hour_12(&self) -> Option<NonZeroU8> {
        self.hour_12
    }

    /// Obtain the `hour_12_is_pm` component.
    pub const fn hour_12_is_pm(&self) -> Option<bool> {
        self.hour_12_is_pm
    }

    /// Obtain the `minute` component.
    pub const fn minute(&self) -> Option<u8> {
        self.minute
    }

    /// Obtain the `second` component.
    pub const fn second(&self) -> Option<u8> {
        self.second
    }

    /// Obtain the `subsecond` component.
    pub const fn subsecond(&self) -> Option<u32> {
        self.subsecond
    }

    /// Obtain the `offset_hour` component.
    pub const fn offset_hour(&self) -> Option<i8> {
        self.offset_hour
    }

    /// Obtain the `offset_minute` component.
    pub const fn offset_minute(&self) -> Option<u8> {
        self.offset_minute
    }

    /// Obtain the `offset_second` component.
    pub const fn offset_second(&self) -> Option<u8> {
        self.offset_second
    }
}

/// Setter methods
///
/// All setters return `Option<()>`, which is `Some` if the value was set, and `None` if not. The
/// setters _may_ fail if the value is invalid, though behavior is not guaranteed.
impl Parsed {
    /// Set the `year` component.
    pub fn set_year(&mut self, value: i32) -> Option<()> {
        self.year = Some(value);
        Some(())
    }

    /// Set the `year_last_two` component.
    pub fn set_year_last_two(&mut self, value: u8) -> Option<()> {
        self.year_last_two = Some(value);
        Some(())
    }

    /// Set the `iso_year` component.
    pub fn set_iso_year(&mut self, value: i32) -> Option<()> {
        self.iso_year = Some(value);
        Some(())
    }

    /// Set the `iso_year_last_two` component.
    pub fn set_iso_year_last_two(&mut self, value: u8) -> Option<()> {
        self.iso_year_last_two = Some(value);
        Some(())
    }

    /// Set the `month` component.
    pub fn set_month(&mut self, value: Month) -> Option<()> {
        self.month = Some(value);
        Some(())
    }

    /// Set the `sunday_week_number` component.
    pub fn set_sunday_week_number(&mut self, value: u8) -> Option<()> {
        self.sunday_week_number = Some(value);
        Some(())
    }

    /// Set the `monday_week_number` component.
    pub fn set_monday_week_number(&mut self, value: u8) -> Option<()> {
        self.monday_week_number = Some(value);
        Some(())
    }

    /// Set the `iso_week_number` component.
    pub fn set_iso_week_number(&mut self, value: NonZeroU8) -> Option<()> {
        self.iso_week_number = Some(value);
        Some(())
    }

    /// Set the `weekday` component.
    pub fn set_weekday(&mut self, value: Weekday) -> Option<()> {
        self.weekday = Some(value);
        Some(())
    }

    /// Set the `ordinal` component.
    pub fn set_ordinal(&mut self, value: NonZeroU16) -> Option<()> {
        self.ordinal = Some(value);
        Some(())
    }

    /// Set the `day` component.
    pub fn set_day(&mut self, value: NonZeroU8) -> Option<()> {
        self.day = Some(value);
        Some(())
    }

    /// Set the `hour_24` component.
    pub fn set_hour_24(&mut self, value: u8) -> Option<()> {
        self.hour_24 = Some(value);
        Some(())
    }

    /// Set the `hour_12` component.
    pub fn set_hour_12(&mut self, value: NonZeroU8) -> Option<()> {
        self.hour_12 = Some(value);
        Some(())
    }

    /// Set the `hour_12_is_pm` component.
    pub fn set_hour_12_is_pm(&mut self, value: bool) -> Option<()> {
        self.hour_12_is_pm = Some(value);
        Some(())
    }

    /// Set the `minute` component.
    pub fn set_minute(&mut self, value: u8) -> Option<()> {
        self.minute = Some(value);
        Some(())
    }

    /// Set the `second` component.
    pub fn set_second(&mut self, value: u8) -> Option<()> {
        self.second = Some(value);
        Some(())
    }

    /// Set the `subsecond` component.
    pub fn set_subsecond(&mut self, value: u32) -> Option<()> {
        self.subsecond = Some(value);
        Some(())
    }

    /// Set the `offset_hour` component.
    pub fn set_offset_hour(&mut self, value: i8) -> Option<()> {
        self.offset_hour = Some(value);
        Some(())
    }

    /// Set the `offset_minute` component.
    pub fn set_offset_minute(&mut self, value: u8) -> Option<()> {
        self.offset_minute = Some(value);
        Some(())
    }

    /// Set the `offset_second` component.
    pub fn set_offset_second(&mut self, value: u8) -> Option<()> {
        self.offset_second = Some(value);
        Some(())
    }
}

/// Builder methods
///
/// All builder methods return `Option<Self>`, which is `Some` if the value was set, and `None` if
/// not. The builder methods _may_ fail if the value is invalid, though behavior is not guaranteed.
impl Parsed {
    /// Set the `year` component and return `self`.
    pub const fn with_year(mut self, value: i32) -> Option<Self> {
        self.year = Some(value);
        Some(self)
    }

    /// Set the `year_last_two` component and return `self`.
    pub const fn with_year_last_two(mut self, value: u8) -> Option<Self> {
        self.year_last_two = Some(value);
        Some(self)
    }

    /// Set the `iso_year` component and return `self`.
    pub const fn with_iso_year(mut self, value: i32) -> Option<Self> {
        self.iso_year = Some(value);
        Some(self)
    }

    /// Set the `iso_year_last_two` component and return `self`.
    pub const fn with_iso_year_last_two(mut self, value: u8) -> Option<Self> {
        self.iso_year_last_two = Some(value);
        Some(self)
    }

    /// Set the `month` component and return `self`.
    pub const fn with_month(mut self, value: Month) -> Option<Self> {
        self.month = Some(value);
        Some(self)
    }

    /// Set the `sunday_week_number` component and return `self`.
    pub const fn with_sunday_week_number(mut self, value: u8) -> Option<Self> {
        self.sunday_week_number = Some(value);
        Some(self)
    }

    /// Set the `monday_week_number` component and return `self`.
    pub const fn with_monday_week_number(mut self, value: u8) -> Option<Self> {
        self.monday_week_number = Some(value);
        Some(self)
    }

    /// Set the `iso_week_number` component and return `self`.
    pub const fn with_iso_week_number(mut self, value: NonZeroU8) -> Option<Self> {
        self.iso_week_number = Some(value);
        Some(self)
    }

    /// Set the `weekday` component and return `self`.
    pub const fn with_weekday(mut self, value: Weekday) -> Option<Self> {
        self.weekday = Some(value);
        Some(self)
    }

    /// Set the `ordinal` component and return `self`.
    pub const fn with_ordinal(mut self, value: NonZeroU16) -> Option<Self> {
        self.ordinal = Some(value);
        Some(self)
    }

    /// Set the `day` component and return `self`.
    pub const fn with_day(mut self, value: NonZeroU8) -> Option<Self> {
        self.day = Some(value);
        Some(self)
    }

    /// Set the `hour_24` component and return `self`.
    pub const fn with_hour_24(mut self, value: u8) -> Option<Self> {
        self.hour_24 = Some(value);
        Some(self)
    }

    /// Set the `hour_12` component and return `self`.
    pub const fn with_hour_12(mut self, value: NonZeroU8) -> Option<Self> {
        self.hour_12 = Some(value);
        Some(self)
    }

    /// Set the `hour_12_is_pm` component and return `self`.
    pub const fn with_hour_12_is_pm(mut self, value: bool) -> Option<Self> {
        self.hour_12_is_pm = Some(value);
        Some(self)
    }

    /// Set the `minute` component and return `self`.
    pub const fn with_minute(mut self, value: u8) -> Option<Self> {
        self.minute = Some(value);
        Some(self)
    }

    /// Set the `second` component and return `self`.
    pub const fn with_second(mut self, value: u8) -> Option<Self> {
        self.second = Some(value);
        Some(self)
    }

    /// Set the `subsecond` component and return `self`.
    pub const fn with_subsecond(mut self, value: u32) -> Option<Self> {
        self.subsecond = Some(value);
        Some(self)
    }

    /// Set the `offset_hour` component and return `self`.
    pub const fn with_offset_hour(mut self, value: i8) -> Option<Self> {
        self.offset_hour = Some(value);
        Some(self)
    }

    /// Set the `offset_minute` component and return `self`.
    pub const fn with_offset_minute(mut self, value: u8) -> Option<Self> {
        self.offset_minute = Some(value);
        Some(self)
    }

    /// Set the `offset_second` component and return `self`.
    pub const fn with_offset_second(mut self, value: u8) -> Option<Self> {
        self.offset_second = Some(value);
        Some(self)
    }
}

impl TryFrom<Parsed> for Date {
    type Error = error::TryFromParsed;

    fn try_from(parsed: Parsed) -> Result<Self, Self::Error> {
        /// Require the items listed.
        macro_rules! items {
            ($($item:ident),+ $(,)?) => {
                Parsed { $($item: Some($item)),*, .. }
            };
        }

        /// Get the value needed to adjust the ordinal day for Sunday and Monday-based week
        /// numbering.
        const fn adjustment(year: i32) -> i16 {
            match Date::__from_ordinal_date_unchecked(year, 1).weekday() {
                Weekday::Monday => 7,
                Weekday::Tuesday => 1,
                Weekday::Wednesday => 2,
                Weekday::Thursday => 3,
                Weekday::Friday => 4,
                Weekday::Saturday => 5,
                Weekday::Sunday => 6,
            }
        }

        // TODO Only the basics have been covered. There are many other valid values that are not
        // currently constructed from the information known.

        match parsed {
            items!(year, ordinal) => Ok(Self::from_ordinal_date(year, ordinal.get())?),
            items!(year, month, day) => Ok(Self::from_calendar_date(year, month, day.get())?),
            items!(iso_year, iso_week_number, weekday) => Ok(Self::from_iso_week_date(
                iso_year,
                iso_week_number.get(),
                weekday,
            )?),
            items!(year, sunday_week_number, weekday) => Ok(Self::from_ordinal_date(
                year,
                (sunday_week_number as i16 * 7 + weekday.number_days_from_sunday() as i16
                    - adjustment(year)
                    + 1) as u16,
            )?),
            items!(year, monday_week_number, weekday) => Ok(Self::from_ordinal_date(
                year,
                (monday_week_number as i16 * 7 + weekday.number_days_from_monday() as i16
                    - adjustment(year)
                    + 1) as u16,
            )?),
            _ => Err(InsufficientInformation),
        }
    }
}

impl TryFrom<Parsed> for Time {
    type Error = error::TryFromParsed;

    fn try_from(parsed: Parsed) -> Result<Self, Self::Error> {
        let hour = match (parsed.hour_24, parsed.hour_12, parsed.hour_12_is_pm) {
            (Some(hour), _, _) => hour,
            (_, Some(hour), Some(false)) if hour.get() == 12 => 0,
            (_, Some(hour), Some(true)) if hour.get() == 12 => 12,
            (_, Some(hour), Some(false)) => hour.get(),
            (_, Some(hour), Some(true)) => hour.get() + 12,
            _ => return Err(InsufficientInformation),
        };
        if parsed.hour_24.is_none()
            && parsed.hour_12.is_some()
            && parsed.hour_12_is_pm.is_some()
            && parsed.minute.is_none()
            && parsed.second.is_none()
            && parsed.subsecond.is_none()
        {
            return Ok(Self::from_hms_nano(hour, 0, 0, 0)?);
        }
        let minute = parsed.minute.ok_or(InsufficientInformation)?;
        let second = parsed.second.unwrap_or(0);
        let subsecond = parsed.subsecond.unwrap_or(0);
        Ok(Self::from_hms_nano(hour, minute, second, subsecond)?)
    }
}

impl TryFrom<Parsed> for UtcOffset {
    type Error = error::TryFromParsed;

    fn try_from(parsed: Parsed) -> Result<Self, Self::Error> {
        let hour = parsed.offset_hour.ok_or(InsufficientInformation)?;
        let minute = parsed.offset_minute.unwrap_or(0);
        let second = parsed.offset_second.unwrap_or(0);
        Self::from_hms(hour, minute as i8, second as i8).map_err(|mut err| {
            // Provide the user a more accurate error.
            if err.name == "hours" {
                err.name = "offset hour";
            } else if err.name == "minutes" {
                err.name = "offset minute";
            } else if err.name == "seconds" {
                err.name = "offset second";
            }
            err.into()
        })
    }
}

impl TryFrom<Parsed> for PrimitiveDateTime {
    type Error = error::TryFromParsed;

    fn try_from(parsed: Parsed) -> Result<Self, Self::Error> {
        Ok(Self::new(parsed.try_into()?, parsed.try_into()?))
    }
}

impl TryFrom<Parsed> for OffsetDateTime {
    type Error = error::TryFromParsed;

    fn try_from(parsed: Parsed) -> Result<Self, Self::Error> {
        Ok(PrimitiveDateTime::try_from(parsed)?.assume_offset(parsed.try_into()?))
    }
}
