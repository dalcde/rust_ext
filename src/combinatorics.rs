pub const MAX_PRIME_INDEX : usize = 54;
const MAX_PRIME : usize = 251;
const NOT_A_PRIME : usize = !1;
const MAX_EXPONENT : usize = 20;
pub const MAX_XI_TAU : usize = 10;
pub const PRIME_TO_INDEX_MAP : [usize;MAX_PRIME+1] = [
    !1, !1, 0, 1, !1, 2, !1, 3, !1, !1, !1, 4, !1, 5, !1, !1, !1, 6, !1, 
    7,  !1, !1, !1, 8, !1, !1, !1, !1, !1, 9, !1, 10, !1, !1, !1, !1, !1, 
    11, !1, !1, !1, 12, !1, 13, !1, !1, !1, 14, !1, !1, !1, !1, !1, 15, 
    !1, !1, !1, !1, !1, 16, !1, 17, !1, !1, !1, !1, !1, 18, !1, !1, !1, 
    19, !1, 20, !1, !1, !1, !1, !1, 21, !1, !1, !1, 22, !1, !1, !1, !1, 
    !1, 23, !1, !1, !1, !1, !1, !1, !1, 24, !1, !1, !1, 25, !1, 26, !1, 
    !1, !1, 27, !1, 28, !1, !1, !1, 29, !1, !1, !1, !1, !1, !1, !1, !1, 
    !1, !1, !1, !1, !1, 30, !1, !1, !1, 31, !1, !1, !1, !1, !1, 32, !1, 
    33, !1, !1, !1, !1, !1, !1, !1, !1, !1, 34, !1, 35, !1, !1, !1, !1, 
    !1, 36, !1, !1, !1, !1, !1, 37, !1, !1, !1, 38, !1, !1, !1, !1, !1, 
    39, !1, !1, !1, !1, !1, 40, !1, 41, !1, !1, !1, !1, !1, !1, !1, !1, 
    !1, 42, !1, 43, !1, !1, !1, 44, !1, 45, !1, !1, !1, !1, !1, !1, !1, 
    !1, !1, !1, !1, 46, !1, !1, !1, !1, !1, !1, !1, !1, !1, !1, !1, 47, 
    !1, !1, !1, 48, !1, 49, !1, !1, !1, 50, !1, !1, !1, !1, !1, 51, !1, 
    52, !1, !1, !1, !1, !1, !1, !1, !1, !1, 53 //, !1, !1, !1, !1
];

static mut INVERSE_TABLE : [Option<Vec<u32>>; MAX_PRIME_INDEX] = [
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None
];

static mut BINOMIAL_TABLE : [Option<Vec<Vec<u32>>>; MAX_PRIME_INDEX] = [
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None
];

static mut XI_DEGREES : [Option<Vec<u32>>; MAX_PRIME_INDEX] = [
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None
];

static mut TAU_DEGREES : [Option<Vec<u32>>; MAX_PRIME_INDEX] = [
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None,None,None,None,None,None,None,
    None,None,None,None
];

pub fn valid_prime_q(p : u32) -> bool {
    (p as usize) < MAX_PRIME && PRIME_TO_INDEX_MAP[p as usize] != NOT_A_PRIME
}

pub fn initialize_prime(p : u32){
    let p_idx = p as usize;
    assert!(valid_prime_q(p));    
    initialize_inverse_table(p);
    initialize_binomial_table(p);
}

fn initialize_inverse_table(p : u32){
    let p_idx = p as usize;
    assert!(valid_prime_q(p));
    let mut table : Vec<u32> = Vec::with_capacity(p_idx);
    for n in 0 .. p {
        table.push(power_mod(p, n, p - 2));
    }
    unsafe { // unsafe for touching mutable static variable        
        INVERSE_TABLE[PRIME_TO_INDEX_MAP[p_idx]] = Some(table);
    }
}

// Finds the inverse of k mod p.
// Uses a the lookup table we initialized.
pub fn inverse(p : u32, k : u32) -> u32{
    assert!(valid_prime_q(p));
    unsafe { // unsafe for touching mutable static variable
        if let Some(table) = &INVERSE_TABLE[PRIME_TO_INDEX_MAP[p as usize]] {
            table[k as usize]
        } else {
            assert!(false);
            0
        }
    }
}

pub fn minus_one_to_the_n(p : u32, i : u32) -> u32 {
    if i % 2 == 0 { 1 } else { p - 1 }
}


// Makes a lookup table for n choose k when n and k are both less than p.
// Lucas's theorem reduces general binomial coefficients to this case.
fn initialize_binomial_table(p : u32){
    let p_idx = p as usize;
    assert!(valid_prime_q(p));  
    unsafe { // unsafe for touching mutable static variable
        if BINOMIAL_TABLE[PRIME_TO_INDEX_MAP[p_idx]] != None {
            return;
        }
    }
    let mut table : Vec<Vec<u32>> = Vec::with_capacity(p_idx);
    for _ in 0 .. p {
        table.push(Vec::with_capacity(p_idx));
    }

    for n in 0 .. p_idx {
        let mut entry = 1usize;
        table[n].push(1);
        for k in 1..n+1 {
            entry *= n + 1 - k;
            entry /= k;
            table[n].push((entry % p_idx) as u32);
        }
        for _ in n+1 .. p_idx {
            table[n].push(0);
        }
    }
    unsafe {
        BINOMIAL_TABLE[PRIME_TO_INDEX_MAP[p_idx]] = Some(table);
    }
}

// This is a table lookup, n, k < p.
fn direct_binomial(p : u32, n : u32, k : u32) -> u32{
    assert!(valid_prime_q(p));
    unsafe{
        if let Some(table) = &BINOMIAL_TABLE[PRIME_TO_INDEX_MAP[p as usize]]{
            table[n as usize][k as usize]
        } else {
            assert!(false);
            0
        }
    }
}


// integer power
// Oftentimes we actually need all powers in a row, so this doesn't get much use.
pub fn integer_power(mut b : u32, mut e : u32) -> u32 {
    let mut result = 1u32;
    while e > 0 {
        if e&1 == 1 {
            result *= b;
        }
        b *= b;
        e >>= 1;
    }
    result
}

// Compute p^b mod e. Same algorithm as above except we reduce mod p after every step.
// We use this for computing modulo inverses.
pub fn power_mod(p : u32, mut b : u32, mut e : u32) -> u32{
    let mut result = 1u32;
//      b is b^{2^i} mod p
//      if the current bit of e is odd, mutliply b^{2^i} mod p into r.
    while e > 0 {
        if (e&1) == 1 {
            result = (result*b)%p;
        }
        b = (b*b)%p;
        e >>= 1;
    }
    result
}

// Discrete log base p of n.
pub fn logp(p : u32, mut n : u32) -> u32 {
    let mut result = 0u32;
    while n > 0 {
        n /= p;
        result += 1;
    }
    result
}

/**
 * Expand n base p and write the result into buffer result.
 * Result has to have length greater than logp(p, n) or we'll have a buffer overflow.
 */
fn basep_expansion(result : &mut[u32], p : u32, mut n : u32) -> &mut[u32] {
    let mut i = 0usize;
    while n > 0 {
        result[i] = n % p;
        i += 1;
        n /= p;
    }
    result
}


//Multinomial coefficient of the list l
fn multinomial2(l : &[u32]) -> u32 {
    let mut bit_or = 0u32;
    let mut sum = 0u32;
    for e in l {
        sum += e;
        bit_or |= e;
//        if(bit_or < sum){
//            return 0;
//        }
    }
    if bit_or == sum { 1 } else { 0 }
}

//Mod 2 binomial coefficient n choose k
fn binomial2(n : i32, k : i32) -> u32 {
    if n < k {
        0
    } else {
        if (n-k) & k == 0 {
            1
        } else {
            0
        }
    }
}

//Mod p multinomial coefficient of l. If p is 2, more efficient to use Multinomial2.
//This uses Lucas's theorem to reduce to n choose k for n, k < p.
fn multinomial_odd(p : u32, l : &[u32]) -> u32{
    let mut total = 0u32;
    for e in l {
        total += e;
    }
    let mut answer = 1u32;
    let mut total_expansion : [u32 ; MAX_EXPONENT] = [0; MAX_EXPONENT];
    let base_p_expansion_length = logp(p, total) as usize;
    basep_expansion(&mut total_expansion, p, total);
    let mut l_expansions : [[u32; MAX_EXPONENT];MAX_XI_TAU] = [[0;MAX_EXPONENT];MAX_XI_TAU];
    for i in 0..l.len() {
        basep_expansion(&mut l_expansions[i], p,  l[i]);
    }
    for index in 0 .. base_p_expansion_length {
        let mut multi = 1u32;
        let mut partial_sum = 0u32;
        for i in 0 .. l.len() {
            partial_sum += l_expansions[i][index];
            if partial_sum > total_expansion[index] {
                return 0
            }
            multi *= direct_binomial(p, partial_sum, l_expansions[i][index]);
            multi = multi % p;
        }
        answer = (answer * multi) % p;
    }
    return answer;
}

//Mod p binomial coefficient n choose k. If p is 2, more efficient to use Binomial2.
fn binomial_odd(p : u32, n : i32, k : i32) -> u32 {
    if n < k || k < 0 {
        return 0;
    }
    let l : [u32 ; 2] = [(n-k) as u32, k as u32];
    return multinomial_odd(p, &l);
}

//Dispatch to Multinomial2 or MultinomialOdd
pub fn multinomial(p : u32, l : &[u32]) -> u32 {
    if p == 2{
        return multinomial2(l);
    } else {
        return multinomial_odd(p, l);
    }
}

//Dispatch to Binomial2 or BinomialOdd
pub fn binomial(p : u32, n : i32, k : i32) -> u32{
    if p == 2{
        return binomial2(n, k);
    } else {
        binomial_odd(p, n, k)
    }
}


pub fn initialize_xi_tau_degrees(p : u32){
    let p_idx = p as usize;
    let mut xi : Vec<u32>= Vec::with_capacity(p_idx);
    let mut tau : Vec<u32>= Vec::with_capacity(p_idx);
    let mut current_xi_degree = 0u32;
    let mut p_to_the_i = 1u32;
    for _ in 0 .. MAX_XI_TAU {
        current_xi_degree += p_to_the_i;
        xi.push(current_xi_degree);
        tau.push(2 * p_to_the_i - 1);
        p_to_the_i *= p;
    }
    unsafe{
        XI_DEGREES[PRIME_TO_INDEX_MAP[p_idx]] = Some(xi);
        TAU_DEGREES[PRIME_TO_INDEX_MAP[p_idx]] = Some(tau);
    }
}


pub fn get_tau_degrees(p : u32) -> &'static [u32] {
    unsafe{
        if let Some(table) = &TAU_DEGREES[PRIME_TO_INDEX_MAP[p as usize]] {
            table
        } else {
            assert!(false);
            return &[]
        }
    }
}

pub fn get_xi_degrees(p : u32) -> &'static [u32] {
    unsafe{
        if let Some(table) = &XI_DEGREES[PRIME_TO_INDEX_MAP[p as usize]] {
            table
        } else {
            assert!(false);
            return &[]
        }
    }
}




#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    // use super::*;

    // #[test]
    // fn test_basep_expansion(){

    //     tables := []struct {
    //         n int
    //         p int
    //         output []int
    //     }{
    //         {8,  3, []int {2, 2}},
    //         {33, 5, []int {3, 1, 1}},
    //     }
    //     for _, table := range tables {
    //         output := basepExpansion(table.n, table.p, 0)
    //         if !eqListsQ(output, table.output) {
    //             t.Errorf("Ran basepExpansion(%v,%v) expected %v got %v", table.n, table.p, table.output, output)
    //         }
    //     }
    // }

    // #[test]
    // fn direct_binomial(t *testing.T) {
    //     tables := []struct {
    //         n int
    //         k int
    //         p int
    //         output int
    //     }{
    //         {21, 2, 23, 210},
    //         {13, 9, 23, 715},
    //         {12, 8, 23, 495},
    //         {13, 8, 23, 1287},
    //         {14, 8, 23, 3003},
    //         {14, 9, 23, 2002},
    //         {15, 5, 23, 3003},
    //         {15, 8, 23, 6435},  
    //         {15, 9, 23, 5005},
    //         {16, 9, 23, 11440},
    //     }
    //     for _, table := range tables {
    //         output := direct_binomial(table.n, table.k, table.p)
    //         if output != table.output  % table.p {
    //             t.Errorf("Ran directBinomial(%v,%v) expected %v, got %v", table.n, table.k, table.output % table.p, output)
    //         }
    //     }    
    // }

    // func TestMultinomial2(t *testing.T) {
    //     tables := []struct {
    //         l []int
    //         output int
    //     }{
    //         {[]int {1, 2}, 1},
    //         {[]int {1, 3}, 0},
    //         {[]int {1, 4}, 1},
    //         {[]int {2, 4}, 1},
    //         {[]int {1, 5}, 0},
    //         {[]int {2, 5}, 1},
    //         {[]int {2, 6}, 0},
    //         {[]int {2, 4, 8}, 1},
    //     }
    //     for _, table := range tables {
    //         output := Multinomial2(table.l)
    //         if output != table.output {
    //             t.Errorf("Ran Multinomial2(%v) expected %v, got %v", table.l, table.output, output)
    //         }
    //     }        
    // }
        
    // func TestBinomial2(t *testing.T) {
    //     tables := []struct {
    //         n int
    //         k int
    //         output int
    //     }{
    //         {4, 2, 0},
    //         {72, 46, 0},
    //         {82, 66, 1},
    //         {165, 132, 1},
    //         {169, 140, 0},
    //     }
    //     for _, table := range tables {
    //         output := Binomial2(table.n, table.k)
    //         if output != table.output {
    //             t.Errorf("Ran Binomial2(%v,%v) expected %v, got %v", table.n, table.k, table.output, output)
    //         }
    //     }        
    // }


    // func TestMultinomialOdd(t *testing.T) {
    //     tables := []struct {
    //         l []int
    //         p int
    //         output int
    //     }{
    //         {[]int {1090, 730}, 3, 1},
    //         {[]int {108054, 758}, 23, 18},
    //         {[]int {3, 2}, 7, 3},
    //     }
    //     for _, table := range tables {
    //         output := MultinomialOdd(table.l, table.p)
    //         if output != table.output {
    //             t.Errorf("Ran MultinomialOdd(%v, %v) expected %v, got %v", table.l, table.p, table.output, output)
    //         }
    //     }        
    // }
    // //
    // func TestBinomialOdd(t *testing.T) {
    
    // }

    // #[test]
    // func TestXiDegrees(t *testing.T) {
    //     let tables : []struct {
    //         n int
    //         p int
    //         output []int
    //     }{
    //         {17,   2, []int{1, 3, 7, 15}},
    //         {17,   3, []int{1, 4, 13}},
    //         {400, 17, []int{1, 18, 307}},
    //     }
        
    //     for _, table := range tables {
    //         output := XiDegrees(table.n, table.p)
    //         if !eqListsQ(output, table.output) {
    //             t.Errorf("Ran XiDegrees(%v, %v) expected %v, got %v", table.n, table.p, table.output, output)
    //         }
    //     }   
    // }
}