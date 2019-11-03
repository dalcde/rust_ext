use crate::actions::*;
use crate::sseq::Sseq;

use bivec::BiVec;
use std::error::Error;

#[cfg(feature = "concurrent")]
use thread_token::TokenBucket;
#[cfg(feature = "concurrent")]
const NUM_THREADS : usize = 2;

const MAX_J : i32 = 18;
use crate::Sender;

const NUM_DELTA = /i;
const MAX_X = MAX_DELTA * 24;

/// This is more-or-less the same as the ResolutionManager, except it manages the Sseq object. The
/// `sender` should send the information to the display frontend.
pub struct SseqManager {
    sender : Sender,
    sseq : Option<Sseq>,
    unit_sseq : Option<Sseq>
}

fn incr(x : &String, n : i32) -> String {
    if n == 0 {
        return x.clone();
    }
    let mut items = x.split(" ");
    let d : i32 = items.next().unwrap().parse().unwrap();
    format!("{} {}", d + n, items.collect::<Vec<_>>().join(" "))
}

impl SseqManager {
    /// Constructs a SseqManager object.
    ///
    /// # Arguments
    ///  * `sender` - The `Sender` object to send messages to.
    pub fn new(sender : Sender) -> Self {
        let mut sseq = Sseq::new(2, SseqChoice::Main, -96, 0, Some(sender.clone()));
        sseq.block_refresh = 1;

        let mut classes : BiVec<BiVec<Vec<String>>> = BiVec::with_capacity(-MAX_X, MAX_X);
        let mut h1: BiVec<BiVec<Vec<String>>> = BiVec::with_capacity(-MAX_X, MAX_X);
        let mut h2: BiVec<BiVec<Vec<String>>> = BiVec::with_capacity(-MAX_X, MAX_X);
        let mut a1: BiVec<BiVec<Vec<String>>> = BiVec::with_capacity(-MAX_X, MAX_X);
        let mut xx: BiVec<BiVec<Vec<String>>> = BiVec::with_capacity(-MAX_X, MAX_X);
        let mut yy: BiVec<BiVec<Vec<String>>> = BiVec::with_capacity(-MAX_X, MAX_X);

        for x in -MAX_X .. MAX_X {
            classes.push(BiVec::with_capacity(0, 20));
            h1.push(BiVec::with_capacity(0, 20));
            h2.push(BiVec::with_capacity(0, 20));
            a1.push(BiVec::with_capacity(0, 20));
            xx.push(BiVec::with_capacity(0, 20));
            yy.push(BiVec::with_capacity(0, 20));
            for _ in 0 .. 20 {
                classes[x].push(Vec::new());
                h1[x].push(Vec::new());
                h2[x].push(Vec::new());
                a1[x].push(Vec::new());
                xx[x].push(Vec::new());
                yy[x].push(Vec::new());
            }
        }

        let bases = [("g1", 0), ("g2", 2), ("g3", 10)];
        for (basis, shift) in bases.into_iter() {
            for d in -NUM_DELTA .. NUM_DELTA {
                let x_shift = d * 24 + shift;
                for j in 0 .. MAX_J - shift / 2 {
                    for i in 0 .. 8 {
                        if x_shift + 2 * j + i >= MAX_X {
                            continue;
                        }
                        classes[x_shift + 2 * j + i][i].push(format!("{} {} a_1^{} h_1^{}", d, basis, j, i));
                        if i < 8 - 1 {
                            h1[x_shift + 2 * j + i][i].push(format!("{} {} a_1^{} h_1^{}", d, basis, j, i + 1));
                        } else {
                            h1[x_shift + 2 * j + i][i].push("".to_string());
                        }
                        if i == 0 && j == 0 {
                            h2[x_shift + 2 * j + i][i].push(format!("{} {} h_2^1", d, basis));
                        } else {
                            h2[x_shift + 2 * j + i][i].push("".to_string());
                        }
                        if j < MAX_J - 1 - shift / 2{
                            a1[x_shift + 2 * j + i][i].push(format!("{} {} a_1^{} h_1^{}", d, basis, j + 1, i));
                        } else {
                            a1[x_shift + 2 * j + i][i].push("".to_string());
                        }
                        if i == 0 && j == 0 {
                            xx[x_shift + 2 * j + i][i].push(format!("{} {} x", d, basis));
                            yy[x_shift + 2 * j + i][i].push(format!("{} {} y", d, basis));
                        } else if i == 1 && j == 0 {
                            xx[x_shift + 2 * j + i][i].push(format!("{} {} h_1 x", d, basis));
                            yy[x_shift + 2 * j + i][i].push(format!("{} {} h_1 y", d, basis));
                        } else if i == 2 && j == 0 {
                            xx[x_shift + 2 * j + i][i].push(format!("{} {} h_2^3", d, basis));
                            yy[x_shift + 2 * j + i][i].push(format!("{} {} h_1^2 y", d, basis));
                        } else if i == 0 && j == 1 {
                            xx[x_shift + 2 * j + i][i].push(format!("{} {} a_1 x", d, basis));
                            yy[x_shift + 2 * j + i][i].push("".to_string());
                        } else if i == 1 && j == 1 {
                            xx[x_shift + 2 * j + i][i].push(format!("{} {} a_1 h_1 x", d, basis));
                            yy[x_shift + 2 * j + i][i].push("".to_string());
                        } else {
                            xx[x_shift + 2 * j + i][i].push("".to_string());
                            yy[x_shift + 2 * j + i][i].push("".to_string());
                        }
                    }
                }
                for i in 1 .. 4 {
                    classes[x_shift + 3 * i][i].push(format!("{} {} h_2^{}", d, basis, i));
                    h1[x_shift + 3 * i][i].push("".to_string());
                    if i < 3 {
                        h2[x_shift + 3 * i][i].push(format!("{} {} h_2^{}", d, basis, i + 1));
                    } else {
                        h2[x_shift + 3 * i][i].push("".to_string());
                    }
                    a1[x_shift + 3 * i][i].push("".to_string());
                    if i == 1 {
                        xx[x_shift + 3 * i][i].push(format!("{} {} a_1 h_1 x", d, basis));
                    } else {
                        xx[x_shift + 3 * i][i].push("".to_string());
                    }
                    if i == 1{
                        yy[x_shift + 3 * i][i].push(format!("{} {} h_2 y", d, basis));
                    } else if i == 2 {
                        yy[x_shift + 3 * i][i].push(format!("{} {} h_2^2 y", d, basis));
                    } else {
                        yy[x_shift + 3 * i][i].push("".to_string());
                    }
                }
                let class_list = vec![
                    // x, y, name, h1, h2, a1, xx
                    (7 , 1, "x"        , "h_1 x"    , "a_1 h_1 x", "a_1 x"    , "d"),
                    (8 , 2, "h_1 x"    , "h_2^3"    , ""         , "a_1 h_1 x", "h_1 d"),
                    (9 , 1, "a_1 x"    , "a_1 h_1 x", ""         , ""         , "h_1 y"),
                    (10, 2, "a_1 h_1 x", ""         , ""         , ""         , "h_1^2 y"),
                    (14, 2, "d"        , "h_1 d"    , "h_1^2 y"  , "h_1 y"    , "h_2^2 y"),
                    (15, 3, "h_1 d"    , ""         , ""         , "h_1^2 y"  , ""),
                    (15, 1, "y"        , "h_1 y"    , "h_2 y"    , ""         , ""),
                    (16, 2, "h_1 y"    , "h_1^2 y"  , ""         , ""         , ""),
                    (17, 3, "h_1^2 y"  , ""         , ""         , ""         , ""),
                    (18, 2, "h_2 y"    , ""         , "h_2^2 y"  , ""         , ""),
                    (21, 3, "h_2^2 y"  , ""         , ""         , ""         , "")
                ];

                for class in class_list {
                    if x_shift + class.0 >= MAX_X {
                        continue;
                    }
                    classes[x_shift + class.0][class.1].push(format!("{} {} {}", d, basis, class.2));
                    h1[x_shift + class.0][class.1].push(if class.3 == "" { "".to_string() } else { format!("{} {} {}", d, basis, class.3) });
                    h2[x_shift + class.0][class.1].push(if class.4 == "" { "".to_string() } else { format!("{} {} {}", d, basis, class.4) });
                    a1[x_shift + class.0][class.1].push(if class.5 == "" { "".to_string() } else { format!("{} {} {}", d, basis, class.5) });
                    xx[x_shift + class.0][class.1].push(if class.6 == "" { "".to_string() } else { format!("{} {} {}", d, basis, class.6) });

                    if class.1 - class.0 == 14 {
                        yy[x_shift + class.0][class.1].push(format!("{} {} a_1^2 h_1^{}", d + 1, basis, class.0 + 1));
                    } else {
                        yy[x_shift + class.0][class.1].push("".to_string());
                    }
                }
            }
        }

        for x in -MAX_X .. MAX_X {
            for y in 0 .. 20 {
                sseq.set_class(x, y as i32, classes[x][y].len());
                for (i, nm) in classes[x][y].iter().enumerate() {
                    sseq.set_class_name(x, y, i, nm.clone());
                }
            }
        }

        // Products
        let tuples = vec![
            (1, 1, "h_1", h1, true),
            (3, 1, "h_2", h2, true),
            (2, 0, "a_1", a1, false),
            (7, 1, "x", xx, false),
            (15, 1, "y", yy, false),
        ];

        let xs = 24;
        let ys = 0;
        let name = "Δ".to_string();
        sseq.add_product_type(&name, xs, ys, true, true);
        for x in -MAX_X .. MAX_X {
            for y in 0 .. 20 {
                if x + xs >= MAX_X || y + ys >= 20 {
                    continue;
                }
                if classes[x][y].len() == 0 {
                    continue;
                }
                let target_dim = classes[x + xs][y + ys].len();
                let mut product_matrix : Vec<Vec<u32>> = Vec::with_capacity(classes[x][y].len());

                for name in &classes[x][y] {
                    let mut row = vec![0; target_dim];

                    let prod = incr(name, 1);

                    let idx = classes[x + xs][y + ys].iter().position(|z| z == &prod).unwrap();
                    row[idx] = 1;

                    product_matrix.push(row)
                }
                assert_eq!(product_matrix.len(), classes[x][y].len());

                sseq.add_product(&name, x, y, xs, ys, true, &product_matrix);
            }
        }

        for (xs, ys, name, arr, perm) in tuples {
            let name = name.to_string();
            sseq.add_product_type(&name, xs, ys, true, perm);

            for x in -MAX_X .. MAX_X {
                for y in 0 .. 20 {
                    if x + xs >= MAX_X || y + ys >= 20 {
                        continue;
                    }
                    if classes[x][y].len() == 0 {
                        continue;
                    }
                    let target_dim = classes[x + xs][y + ys].len();
                    let mut product_matrix : Vec<Vec<u32>> = Vec::with_capacity(classes[x][y].len());

                    for prod in &arr[x][y] {
                        let mut row = vec![0; target_dim];

                        if prod != "" {
                            let idx = classes[x + xs][y + ys].iter().position(|z| z == prod).unwrap();
                            row[idx] = 1;
                        }
                        product_matrix.push(row)
                    }
                    assert_eq!(product_matrix.len(), classes[x][y].len());

                    sseq.add_product(&name, x, y, xs, ys, true, &product_matrix);
                }
            }
        }
        let square = [("h_2", true), ("a_1", false), ("x", true)];

        for (name, perm) in &square {
            let name = name.to_string();
            let new_name = format!("{}^2", name);

            let prod_idx = *sseq.product_name_to_index.get(&name).unwrap();
            let prod_obj = sseq.products.read().unwrap();
            let product = &prod_obj[prod_idx];

            let xs = product.x;
            let ys = product.y;

            drop(product);
            drop(prod_obj);

            sseq.add_product_type(&new_name, xs * 2, ys * 2, true, *perm);

            let product = &mut sseq.products.write().unwrap();
            let old_prod_idx = prod_idx;
            let new_prod_idx = *sseq.product_name_to_index.get(&new_name).unwrap();

            for x in -MAX_X .. MAX_X {
                for y in 0 .. 20 {
                    if x + xs * 2 >= MAX_X || y + ys * 2 >= 20 {
                        continue;
                    }

                    if classes[x][y].len() == 0 || classes[x + xs][y + ys].len() == 0 || classes[x + xs * 2][y + ys * 2].len() == 0 {
                        continue;
                    }

                    if product[old_prod_idx].matrices[x][y].is_none() || product[old_prod_idx].matrices[x + xs][y + ys].is_none() {
                        continue;
                    }

                    let result = product[old_prod_idx].matrices[x][y].as_ref().unwrap() * product[old_prod_idx].matrices[x + xs][y + ys].as_ref().unwrap();

                    while x >= product[new_prod_idx].matrices.len() {
                        product[new_prod_idx].matrices.push(BiVec::new(sseq.min_y));
                    }
                    while y > product[new_prod_idx].matrices[x].len() {
                        product[new_prod_idx].matrices[x].push(None);
                    }

                    product[new_prod_idx].matrices[x].push(Some(result));
                }
            }
        }

        sseq.add_product_differential_r(&"a_1".to_string(), &"h_1".to_string(), 1);
        sseq.add_product_differential_r(&"x".to_string(), &"h_2^2".to_string(), 1);
        sseq.add_product_differential_r(&"a_1^2".to_string(), &"h_2".to_string(), 2);
        sseq.add_product_differential_r(&"y".to_string(), &"x^2".to_string(), 1);

        sseq.block_refresh = 0;
        sseq.refresh_all();

        SseqManager {
             sender : sender,
             sseq : Some(sseq),
             unit_sseq : None
        }
   }

    /// # Return
    /// Whether this was a user action. If it is a user action, we want to send a "Complete" when
    /// completed, and also report the time.
    pub fn is_user(action : &Action) -> bool{
        match action {
            Action::AddClass(_) => false,
            Action::AddProduct(_) => false,
            Action::Complete(_) => false,
            Action::QueryTableResult(_) => false,
            Action::QueryCocycleStringResult(_) => false,
            Action::Resolving(_) => false,
            _ => true
        }
    }

    pub fn process_message(&mut self, msg : Message) -> Result<bool, Box<dyn Error>> {
        let user = Self::is_user(&msg.action);
        let target_sseq = msg.sseq;

        match msg.action {
            Action::Resolving(_) => self.resolving(msg)?,
            Action::Complete(_) => self.relay(msg)?,
            Action::QueryTableResult(_) => self.relay(msg)?,
            Action::QueryCocycleStringResult(_) => self.relay(msg)?,
            _ => {
                if let Some(sseq) = self.get_sseq(msg.sseq) {
                    msg.action.act_sseq(sseq);
                }
            }
        };

        if user {
            self.sender.send(Message {
                recipients : vec![],
                sseq : target_sseq,
                action : Action::from(Complete {})
            })?;
        }
        Ok(user)
    }

    fn get_sseq(&mut self, sseq : SseqChoice) -> Option<&mut Sseq> {
        match sseq {
            SseqChoice::Main => self.sseq.as_mut(),
            SseqChoice::Unit => self.unit_sseq.as_mut()
        }
    }

    fn resolving(&mut self, _msg : Message) -> Result<(), Box<dyn Error>> {
        panic!();
    }

    fn relay(&self, msg : Message) -> Result<(), Box<dyn Error>> {
        self.sender.send(msg)?;
        Ok(())
    }
}
