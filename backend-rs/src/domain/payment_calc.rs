use rust_decimal::{Decimal, RoundingStrategy};
use rust_decimal_macros::dec;

/// Fallback only — not every employee is cut at this rate (see `employees.pay_rate_percent`).
pub const DEFAULT_PAY_RATE_PERCENT: Decimal = dec!(42);

#[derive(Clone, Copy, Debug)]
pub struct LoadAmounts {
    pub pay: Decimal,
    pub extra_pay: Decimal,
}

/// Round-half-up to the nearest whole dollar: .50+ rounds up, .49 and below rounds down.
/// Amounts are never negative (DB CHECKs), so away-from-zero == half-up here. Never use
/// `Decimal::round()` — that's banker's rounding.
pub fn round_to_dollar(amount: Decimal) -> Decimal {
    amount.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
}

pub struct EmployeeCut {
    pub employee_cut: Decimal,
    pub rounded_pay: Decimal,
}

pub fn calculate_employee_cut(summary_total: Decimal, pay_rate_percent: Decimal) -> EmployeeCut {
    let employee_cut = summary_total * pay_rate_percent / Decimal::ONE_HUNDRED;
    EmployeeCut {
        employee_cut,
        rounded_pay: round_to_dollar(employee_cut),
    }
}

pub fn sum_loads(loads: &[LoadAmounts]) -> Decimal {
    loads.iter().map(|load| load.pay + load.extra_pay).sum()
}

// Mirrors backend/unit-tests/payment-calc.test.ts and frontend's payment-calc tests — keep
// the shared cases in sync.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_fifty_cents_up() {
        assert_eq!(round_to_dollar(dec!(120.50)), dec!(121));
    }

    #[test]
    fn rounds_forty_nine_cents_down() {
        assert_eq!(round_to_dollar(dec!(120.49)), dec!(120));
    }

    #[test]
    fn rounds_zero() {
        assert_eq!(round_to_dollar(Decimal::ZERO), dec!(0));
    }

    #[test]
    fn does_not_use_bankers_rounding() {
        // Banker's rounding would give 120 (round half to even).
        assert_eq!(round_to_dollar(dec!(120.5)), dec!(121));
        assert_eq!(round_to_dollar(dec!(121.5)), dec!(122));
    }

    #[test]
    fn applies_default_cut_and_rounds() {
        let cut = calculate_employee_cut(dec!(1000), DEFAULT_PAY_RATE_PERCENT);
        assert_eq!(cut.employee_cut, dec!(420));
        assert_eq!(cut.rounded_pay, dec!(420));
    }

    #[test]
    fn applies_per_employee_rate() {
        let cut = calculate_employee_cut(dec!(1001.25), dec!(40));
        assert_eq!(cut.employee_cut, dec!(400.5));
        assert_eq!(cut.rounded_pay, dec!(401));
    }

    #[test]
    fn sums_pay_and_extra_pay() {
        let total = sum_loads(&[
            LoadAmounts {
                pay: dec!(100),
                extra_pay: dec!(10),
            },
            LoadAmounts {
                pay: dec!(50),
                extra_pay: dec!(0),
            },
        ]);
        assert_eq!(total, dec!(160));
    }

    #[test]
    fn sums_without_float_drift() {
        let total = sum_loads(&[
            LoadAmounts {
                pay: dec!(0.1),
                extra_pay: dec!(0),
            },
            LoadAmounts {
                pay: dec!(0.2),
                extra_pay: dec!(0),
            },
        ]);
        assert_eq!(total, dec!(0.3));
    }
}
