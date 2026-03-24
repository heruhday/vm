                Opcode::AddI32 => {
                    // Fast path: int32 + int32
                    let lhs = self.frame.regs[b];
                    let rhs = self.frame.regs[c];
                    
                    // Check if both are ints
                    if lhs.is_int() && rhs.is_int() {
                        let a_int = lhs.int_payload_unchecked();
                        let b_int = rhs.int_payload_unchecked();
                        if let Some(result) = a_int.checked_add(b_int) {
                            self.frame.regs[ACC] = make_int32(result);
                            if a != ACC {
                                self.frame.regs[a] = make_int32(result);
                            }
                            continue;
                        }
                    }
                    // Fall back to slow path
                    let (lhs, rhs) = self.value_pair(lhs, rhs);
                    self.frame.regs[ACC] = lhs.add(&rhs).raw();
                    if a != ACC {
                        self.frame.regs[a] = self.frame.regs[ACC];
                    }
                }
                Opcode::AddF64 => {
                    // Fast path: f64 + f64
                    let lhs = self.frame.regs[b];
                    let rhs = self.frame.regs[c];
                    
                    // Check if both are f64
                    if lhs.is_f64() && rhs.is_f64() {
                        let a_f64 = lhs.f64_payload_unchecked();
                        let b_f64 = rhs.f64_payload_unchecked();
                        self.frame.regs[ACC] = make_number(a_f64 + b_f64);
                        if a != ACC {
                            self.frame.regs[a] = self.frame.regs[ACC];
                        }
                        continue;
                    }
                    // Fall back to slow path
                    let (lhs, rhs) = self.value_pair(lhs, rhs);
                    self.frame.regs[ACC] = lhs.add(&rhs).raw();
                    if a != ACC {
                        self.frame.regs[a] = self.frame.regs[ACC];
                    }
                }
                Opcode::SubI32 => {
                    // Fast path: int32 - int32
                    let lhs = self.frame.regs[b];
                    let rhs = self.frame.regs[c];
                    
                    if lhs.is_int() && rhs.is_int() {
                        let a_int = lhs.int_payload_unchecked();
                        let b_int = rhs.int_payload_unchecked();
                        if let Some(result) = a_int.checked_sub(b_int) {
                            self.frame.regs[ACC] = make_int32(result);
                            if a != ACC {
                                self.frame.regs[a] = make_int32(result);
                            }
                            continue;
                        }
                    }
                    // Fall back to slow path
                    let (lhs, rhs) = self.value_pair(lhs, rhs);
                    self.frame.regs[ACC] = lhs.sub(&rhs).raw();
                    if a != ACC {
                        self.frame.regs[a] = self.frame.regs[ACC];
                    }
                }
                Opcode::SubF64 => {
                    // Fast path: f64 - f64
                    let lhs = self.frame.regs[b];
                    let rhs = self.frame.regs[c];
                    
                    if lhs.is_f64() && rhs.is_f64() {
                        let a_f64 = lhs.f64_payload_unchecked();
                        let b_f64 = rhs.f64_payload_unchecked();
                        self.frame.regs[ACC] = make_number(a_f64 - b_f64);
                        if a != ACC {
                            self.frame.regs[a] = self.frame.regs[ACC];
                        }
                        continue;
                    }
                    // Fall back to slow path
                    let (lhs, rhs) = self.value_pair(lhs, rhs);
                    self.frame.regs[ACC] = lhs.sub(&rhs).raw();
                    if a != ACC {
                        self.frame.regs[a] = self.frame.regs[ACC];
                    }
                }
                Opcode::MulI32 => {
                    // Fast path: int32 * int32
                    let lhs = self.frame.regs[b];
                    let rhs = self.frame.regs[c];
                    
                    if lhs.is_int() && rhs.is_int() {
                        let a_int = lhs.int_payload_unchecked();
                        let b_int = rhs.int_payload_unchecked();
                        if let Some(result) = a_int.checked_mul(b_int) {
                            self.frame.regs[ACC] = make_int32(result);
                            if a != ACC {
                                self.frame.regs[a] = make_int32(result);
                            }
                            continue;
                        }
                    }
                    // Fall back to slow path
                    let (lhs, rhs) = self.value_pair(lhs, rhs);
                    self.frame.regs[ACC] = lhs.mul(&rhs).raw();
                    if a != ACC {
                        self.frame.regs[a] = self.frame.regs[ACC];
                    }
                }
                Opcode::MulF64 => {
                    // Fast path: f64 * f64
                    let lhs = self.frame.regs[b];
                    let rhs = self.frame.regs[c];
                    
                    if lhs.is_f64() && rhs.is_f64() {
                        let a_f64 = lhs.f64_payload_unchecked();
                        let b_f64 = rhs.f64_payload_unchecked();
                        self.frame.regs[ACC] = make_number(a_f64 * b_f64);
                        if a != ACC {
                            self.frame.regs[a] = self.frame.regs[ACC];
                        }
                        continue;
                    }
                    // Fall back to slow path
                    let (lhs, rhs) = self.value_pair(lhs, rhs);
                    self.frame.regs[ACC] = lhs.mul(&rhs).raw();
                    if a != ACC {
                        self.frame.regs[a] = self.frame.regs[ACC];
                    }
                }