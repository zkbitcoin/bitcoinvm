use crate::ripemd160::table16::{AssignedBits};
use crate::ripemd160::table16::spread_table::{SpreadInputs, SpreadVar, SpreadWord};
use crate::ripemd160::table16::util::{i2lebsp, even_bits, odd_bits, lebs2ip, negate_spread};

use super::{CompressionConfig, RoundWordSpread};

use halo2::{
    circuit::{Region, Value},
    plonk::{Advice, Column, Error},
};
use halo2::halo2curves::pasta::pallas;
use std::convert::TryInto;





impl CompressionConfig {

    // s_f1 | a_0 |   a_1    |       a_2       |    a_3      |    a_4      |    a_5      |
    //   1  |     | R_0_even | spread_R_0_even | spread_B_lo | spread_C_lo | spread_D_lo | 
    //      |     | R_0_odd  | spread_R_0_odd  | spread_B_hi | spread_C_hi | spread_D_hi | 
    //      |     | R_1_even | spread_R_1_even |             |             |             | 
    //      |     | R_1_odd  | spread_R_1_odd  |             |             |             | 
    // 
    pub(super) fn assign_f1(
        &self,
        region: &mut Region<'_, pallas::Base>,
        row: usize,
        spread_halves_b: RoundWordSpread,
        spread_halves_c: RoundWordSpread,
        spread_halves_d: RoundWordSpread,
    ) -> Result<(AssignedBits<16>, AssignedBits<16>), Error> {
        let a_3 = self.advice[0];
        let a_4 = self.advice[1];
        let a_5 = self.advice[2];
        
        self.s_f1.enable(region, row)?;

        // Assign and copy spread_b_lo, spread_b_hi
        spread_halves_b.0.copy_advice(|| "spread_b_lo", region, a_3, row)?;
        spread_halves_b.1.copy_advice(|| "spread_b_hi", region, a_3, row + 1)?;

        // Assign and copy spread_c_lo, spread_c_hi
        spread_halves_c.0.copy_advice(|| "spread_c_lo", region, a_4, row)?;
        spread_halves_c.1.copy_advice(|| "spread_c_hi", region, a_4, row + 1)?;

        // Assign and copy spread_d_lo, spread_d_hi
        spread_halves_d.0.copy_advice(|| "spread_d_lo", region, a_5, row)?;
        spread_halves_d.1.copy_advice(|| "spread_d_hi", region, a_5, row + 1)?;

        let m: Value<[bool; 64]> = spread_halves_b
            .value()
            .zip(spread_halves_c.value())
            .zip(spread_halves_d.value())
            .map(|((a, b), c)| i2lebsp(a + b + c));

        let r_0: Value<[bool; 32]> = m.map(|m| m[..32].try_into().unwrap());
        let r_0_even = r_0.map(even_bits);
        let r_0_odd = r_0.map(odd_bits);

        let r_1: Value<[bool; 32]> = m.map(|m| m[32..].try_into().unwrap());
        let r_1_even = r_1.map(even_bits);
        let r_1_odd = r_1.map(odd_bits);

        self.assign_f1_outputs(region, row, r_0_even, r_0_odd, r_1_even, r_1_odd)
    }

    fn assign_f1_outputs(
        &self,
        region: &mut Region<'_, pallas::Base>,
        row: usize,
        r_0_even: Value<[bool; 16]>,
        r_0_odd: Value<[bool; 16]>,
        r_1_even: Value<[bool; 16]>,
        r_1_odd: Value<[bool; 16]>,
    ) -> Result<(AssignedBits<16>, AssignedBits<16>), Error> {
        let (even, _odd) = self.assign_spread_outputs(
            region,
            &self.lookup,
            row,
            r_0_even,
            r_0_odd,
            r_1_even,
            r_1_odd,
        )?;

        Ok(even)
    }

    // s_ch | a_0 |   a_1    |       a_2       |    a_3      |    a_4      |    a_5      |
    //   1  |     | P_0_even | spread_P_0_even | spread_X_lo | spread_Y_lo |             | 
    //      |     | P_0_odd  | spread_P_0_odd  | spread_X_hi | spread_Y_hi |             | 
    //      |     | P_1_even | spread_P_1_even |             |             |             | 
    //      |     | P_1_odd  | spread_P_1_odd  |             |             |             | 
    // 
    pub(super) fn assign_ch(
        &self,
        region: &mut Region<'_, pallas::Base>,
        row: usize,
        spread_halves_x: RoundWordSpread,
        spread_halves_y: RoundWordSpread,
    ) -> Result<(AssignedBits<16>, AssignedBits<16>), Error> {
        let a_3 = self.advice[0];
        let a_4 = self.advice[1];

        self.s_ch.enable(region, row)?;

        // Assign and copy spread_x_lo, spread_x_hi
        spread_halves_x.0.copy_advice(|| "spread_x_lo", region, a_3, row)?;
        spread_halves_x.1.copy_advice(|| "spread_x_hi", region, a_3, row + 1)?;

        // Assign and copy spread_y_lo, spread_y_hi
        spread_halves_y.0.copy_advice(|| "spread_y_lo", region, a_4, row)?;
        spread_halves_y.1.copy_advice(|| "spread_y_hi", region, a_4, row + 1)?;

        let p: Value<[bool; 64]> = spread_halves_x
            .value()
            .zip(spread_halves_y.value())
            .map(|(e, f)| i2lebsp(e + f));

        let p_0: Value<[bool; 32]> = p.map(|p| p[..32].try_into().unwrap());
        let p_0_even = p_0.map(even_bits);
        let p_0_odd = p_0.map(odd_bits);

        let p_1: Value<[bool; 32]> = p.map(|p| p[32..].try_into().unwrap());
        let p_1_even = p_1.map(even_bits);
        let p_1_odd = p_1.map(odd_bits);

        self.assign_ch_outputs(region, row, p_0_even, p_0_odd, p_1_even, p_1_odd)
    }

    // s_ch_neg | a_0 |   a_1    |       a_2       |    a_3          |    a_4      |    a_5      |
    //   1      |     | Q_0_even | spread_Q_0_even | spread_neg_X_lo | spread_Z_lo | spread_X_lo | 
    //          |     | Q_0_odd  | spread_Q_0_odd  | spread_neg_X_hi | spread_Z_hi | spread_X_hi | 
    //          |     | Q_1_even | spread_Q_1_even |                 |             |             | 
    //          |     | Q_1_odd  | spread_Q_1_odd  |                 |             |             | 
    // 
    pub(super) fn assign_ch_neg(
        &self,
        region: &mut Region<'_, pallas::Base>,
        row: usize,
        spread_halves_x: RoundWordSpread,
        spread_halves_z: RoundWordSpread,
    ) -> Result<(AssignedBits<16>, AssignedBits<16>), Error> {
        let a_3 = self.advice[0];
        let a_4 = self.advice[1];
        let a_5 = self.advice[2];

        self.s_ch_neg.enable(region, row)?;

        // Assign and copy spread_x_lo, spread_x_hi
        spread_halves_x.0.copy_advice(|| "spread_x_lo", region, a_3, row)?;
        spread_halves_x.1.copy_advice(|| "spread_x_hi", region, a_3, row + 1)?;

        // Assign and copy spread_z_lo, spread_z_hi
        spread_halves_z.0.copy_advice(|| "spread_z_lo", region, a_4, row)?;
        spread_halves_z.1.copy_advice(|| "spread_z_hi", region, a_4, row + 1)?;

        // Calculate neg_x_lo
        let spread_neg_x_lo = spread_halves_x
            .0
            .value()
            .map(|spread_x_lo| negate_spread(spread_x_lo.0));
        // Assign spread_neg_x_lo
        AssignedBits::<32>::assign_bits(
            region,
            || "spread_neg_x_lo",
            a_5,
            row,
            spread_neg_x_lo,
        )?;

        // Calculate neg_x_hi
        let spread_neg_x_hi = spread_halves_x
            .1
            .value()
            .map(|spread_x_hi| negate_spread(spread_x_hi.0));
        // Assign spread_neg_x_hi
        AssignedBits::<32>::assign_bits(
            region,
            || "spread_neg_x_hi",
            a_5,
            row + 1,
            spread_neg_x_hi,
        )?;

        let q: Value<[bool; 64]> = {
            let spread_neg_x = spread_neg_x_lo
                .zip(spread_neg_x_hi)
                .map(|(lo, hi)| lebs2ip(&lo) + (1 << 32) * lebs2ip(&hi));
            spread_neg_x
                .zip(spread_halves_z.value())
                .map(|(neg_x, z)| i2lebsp(neg_x + z))
        };

        let q_0: Value<[bool; 32]> = q.map(|q| q[..32].try_into().unwrap());
        let q_0_even = q_0.map(even_bits);
        let q_0_odd = q_0.map(odd_bits);

        let q_1: Value<[bool; 32]> = q.map(|q| q[32..].try_into().unwrap());
        let q_1_even = q_1.map(even_bits);
        let q_1_odd = q_1.map(odd_bits);

        self.assign_ch_outputs(region, row, q_0_even, q_0_odd, q_1_even, q_1_odd)
    }

    fn assign_ch_outputs(
        &self,
        region: &mut Region<'_, pallas::Base>,
        row: usize,
        p_0_even: Value<[bool; 16]>,
        p_0_odd: Value<[bool; 16]>,
        p_1_even: Value<[bool; 16]>,
        p_1_odd: Value<[bool; 16]>,
    ) -> Result<(AssignedBits<16>, AssignedBits<16>), Error> {
        let (_even, odd) = self.assign_spread_outputs(
            region,
            &self.lookup,
            row,
            p_0_even,
            p_0_odd,
            p_1_even,
            p_1_odd,
        )?;

        Ok(odd)
    }

    //          | a_0 |   a_1    |       a_2       |
    // row      |     | R_0_even | spread_R_0_even | 
    // row + 1  |     | R_0_odd  | spread_R_0_odd  | 
    // row + 2  |     | R_1_even | spread_R_1_even | 
    // row + 3  |     | R_1_odd  | spread_R_1_odd  | 
    // 
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn assign_spread_outputs(
        &self,
        region: &mut Region<'_, pallas::Base>,
        lookup: &SpreadInputs,
        row: usize,
        r_0_even: Value<[bool; 16]>,
        r_0_odd: Value<[bool; 16]>,
        r_1_even: Value<[bool; 16]>,
        r_1_odd: Value<[bool; 16]>,
    ) -> Result<
        (
            (AssignedBits<16>, AssignedBits<16>),
            (AssignedBits<16>, AssignedBits<16>),
        ),
        Error,
    > {
        // Lookup R_0^{even}, R_0^{odd}, R_1^{even}, R_1^{odd}
        let r_0_even = SpreadVar::with_lookup(
            region,
            lookup,
            row,
            r_0_even.map(SpreadWord::<16, 32>::new),
        )?;
        let r_0_odd = SpreadVar::with_lookup(
            region,
            lookup,
            row + 1,
            r_0_odd.map(SpreadWord::<16, 32>::new),
        )?;
        let r_1_even = SpreadVar::with_lookup(
            region,
            lookup,
            row + 2,
            r_1_even.map(SpreadWord::<16, 32>::new),
        )?;
        let r_1_odd = SpreadVar::with_lookup(
            region,
            lookup,
            row + 3,
            r_1_odd.map(SpreadWord::<16, 32>::new),
        )?;

        Ok((
            (r_0_even.dense, r_1_even.dense),
            (r_0_odd.dense, r_1_odd.dense),
        ))
    }

}