import Mathlib.Tactic.Ring

/-!
# Hydro · the wire codec

Binary codecs for what crosses between processes — the mirror of the Rust
message structs' `#[derive(Serialize, Deserialize)]` under `bincode`: a
value is its fields in order, naturals as LEB128 varints, tags as one byte,
lists length-prefixed. **A `Wire` instance carries its round-trip law**: reading
the value's bytes back, at any offset in any buffer, yields the value and the
offset after it. The law is what makes the byte-level run of a program the
typed run (`Dag.iterateWire`, `paxos_safe_wired` — D88): a wrong codec is
a failed proof, not a dropped message.

The containers are supplied here; the Rust message structs (`Ballot`, `P1b`, …)
are `Wire.ofEquiv` of their field tuples in `Paxos/Types.lean`, `KvReplica.lean`,
`BenchClient.lean`, `PaxosBench.lean`.

(JSON was the first wire, D74–D85; parsing and printing it was the leader's
whole decode budget at ten thousand messages a second — D87.)
-/

namespace Hydro

/-- A wire form: a value's bytes, and reading one back at an offset (the next
offset returned) — with the law that reading what was written gives it back. -/
class Wire (α : Type) where
  encode : α → ByteArray
  get : ByteArray → Nat → Except String (α × Nat)
  get_encode : ∀ (x : α) (pre suf : ByteArray),
    get (pre ++ encode x ++ suf) pre.size = .ok (x, pre.size + (encode x).size)
  /-- The value's bytes written onto a buffer — the encoder a codec runs (`Codec.ofWire`).
  `encode` is the specification (a fresh array per field, `++` copying them together:
  framing one message copied its payload four times, D92); an instance's `put` writes the
  fields in place and `put_eq` is its proof. The default is the specification itself. -/
  put : α → ByteArray → ByteArray := fun x b => b ++ encode x
  put_eq : ∀ (x : α) (b : ByteArray), put x b = b ++ encode x := by intros; rfl

namespace Wire

/-- A value's bytes through `put` (`= encode x`, `putEmpty_eq`). -/
def putEmpty {α : Type} [Wire α] (x : α) : ByteArray := put x (ByteArray.emptyWithCapacity 64)

theorem putEmpty_eq {α : Type} [Wire α] (x : α) : putEmpty x = encode x := by
  unfold putEmpty
  rw [put_eq]
  exact ByteArray.empty_append

/-- A whole value from exactly its bytes. -/
def decode {α : Type} [Wire α] (b : ByteArray) : Except String α := do
  let (a, i) ← get (α := α) b 0
  if i = b.size then .ok a else .error s!"wire: {b.size - i} trailing bytes"

theorem decode_encode {α : Type} [Wire α] (x : α) : decode (encode x) = .ok x := by
  have h := get_encode x ByteArray.empty ByteArray.empty
  rw [ByteArray.empty_append, ByteArray.append_empty, ByteArray.size_empty, Nat.zero_add] at h
  simp only [decode, h, bind, Except.bind, if_true]

/-- A wire form through a representation: `α` as `β` (`f`), back by `g`. -/
@[instance_reducible] def ofEquiv {α β : Type} [Wire β] (f : α → β) (g : β → α) (hgf : ∀ x, g (f x) = x) : Wire α where
  encode x := encode (f x)
  get b i := (get (α := β) b i).map fun (y, i) => (g y, i)
  get_encode x pre suf := by
    show (get (α := β) (pre ++ encode (f x) ++ suf) pre.size).map _ = _
    rw [get_encode (f x) pre suf]
    simp only [Except.map, hgf]
  put x b := put (f x) b
  put_eq x b := put_eq (f x) b

/-! ### Naturals: LEB128, seven bits a byte, high bit = more -/

def natBytes (n : Nat) : List UInt8 :=
  if h : n < 128 then [n.toUInt8]
  else (n % 128 + 128).toUInt8 :: natBytes (n / 128)
termination_by n
decreasing_by omega

def getNat (b : ByteArray) (i : Nat) (acc : Nat := 0) (shift : Nat := 0) : Except String (Nat × Nat) :=
  if h : i < b.size then
    let x := (b[i]'h).toNat
    if x < 128 then .ok (acc + (x <<< shift), i + 1)
    else getNat b (i + 1) (acc + ((x - 128) <<< shift)) (shift + 7)
  else .error "wire: truncated natural"
termination_by b.size - i

/-! `getNat` compiled: the runtime's `Nat.shiftLeft` (`lean_nat_shiftl`) builds a GMP
integer even for small operands — 7 % of the benchmark's cycles were `<<<` in the decoder
(D92). The compiled form carries the multiplier `2 ^ shift` and multiplies (`lean_nat_mul`
stays scalar); `getNat_eq_impl` is the `@[csimp]` equation. -/

def getNatImpl (b : ByteArray) (i : Nat) (acc mult : Nat) : Except String (Nat × Nat) :=
  if h : i < b.size then
    let x := (b[i]'h).toNat
    if x < 128 then .ok (acc + x * mult, i + 1)
    else getNatImpl b (i + 1) (acc + (x - 128) * mult) (mult * 128)
  else .error "wire: truncated natural"
termination_by b.size - i

/-- `2 ^ n` without `Nat.pow` (also a GMP call in the runtime). -/
def pow2 : Nat → Nat
  | 0 => 1
  | n + 1 => 2 * pow2 n

theorem pow2_eq (n : Nat) : pow2 n = 2 ^ n := by
  induction n with
  | zero => rfl
  | succ n ih => rw [pow2, ih, Nat.pow_succ, Nat.mul_comm]

theorem getNat_eq_getNatImpl (b : ByteArray) (i acc shift : Nat) :
    getNat b i acc shift = getNatImpl b i acc (2 ^ shift) := by
  induction i, acc, shift using getNat.induct b with
  | case1 i acc shift h x hx =>
    rw [getNat, getNatImpl, dif_pos h, dif_pos h]
    simp only [x] at hx
    simp only [hx, if_true, Nat.shiftLeft_eq]
  | case2 i acc shift h x hx ih =>
    rw [getNat, getNatImpl, dif_pos h, dif_pos h]
    simp only [x] at hx ih
    simp only [hx, if_false, Nat.shiftLeft_eq, Nat.pow_add] at ih ⊢
    exact ih
  | case3 i acc shift h =>
    rw [getNat, getNatImpl, dif_neg h, dif_neg h]

def getNatFrom (b : ByteArray) (i : Nat) (acc : Nat := 0) (shift : Nat := 0) : Except String (Nat × Nat) :=
  getNatImpl b i acc (pow2 shift)

@[csimp] theorem getNat_eq_impl : @getNat = @getNatFrom := by
  funext b i acc shift
  rw [getNatFrom, pow2_eq, getNat_eq_getNatImpl]

/-- The byte at `pre.size + k` of `pre ++ mid ++ suf` is `mid`'s `k`-th. -/
theorem getElem_append_mid (pre mid suf : ByteArray) (k : Nat) (hk : k < mid.size)
    (h : pre.size + k < (pre ++ mid ++ suf).size) :
    (pre ++ mid ++ suf)[pre.size + k]'h = mid[k]'hk := by
  rw [ByteArray.getElem_append_left (by rw [ByteArray.size_append]; omega),
    ByteArray.getElem_append_right (Nat.le_add_right _ _)]
  simp

theorem size_append_mid (pre mid suf : ByteArray) : (pre ++ mid ++ suf).size = pre.size + mid.size + suf.size := by
  rw [ByteArray.size_append, ByteArray.size_append]

theorem toNat_toUInt8 (k : Nat) (hk : k < 256) : (k.toUInt8).toNat = k := by
  simp [Nat.toUInt8]; omega

theorem shift_split (acc n shift : Nat) :
    acc + ((n % 128 + 128 - 128) <<< shift) + ((n / 128) <<< (shift + 7)) = acc + (n <<< shift) := by
  rw [Nat.add_sub_cancel]
  simp only [Nat.shiftLeft_eq, Nat.pow_add]
  have h := Nat.mod_add_div n 128
  calc acc + n % 128 * 2 ^ shift + n / 128 * (2 ^ shift * 2 ^ 7)
      = acc + (n % 128 + 128 * (n / 128)) * 2 ^ shift := by ring
    _ = acc + n * 2 ^ shift := by rw [h]

/-- One step of `getNat` at a readable offset. -/
theorem getNat_step (b : ByteArray) (i acc shift : Nat) (h : i < b.size) :
    getNat b i acc shift
      = if (b[i]'h).toNat < 128 then .ok (acc + ((b[i]'h).toNat <<< shift), i + 1)
        else getNat b (i + 1) (acc + (((b[i]'h).toNat - 128) <<< shift)) (shift + 7) := by
  rw [getNat, dif_pos h]

/-- LEB128 reads back what it wrote, at any offset, into any accumulator. -/
theorem getNat_natBytes (n : Nat) : ∀ (pre suf : ByteArray) (acc shift : Nat),
    getNat (pre ++ (natBytes n).toByteArray ++ suf) pre.size acc shift
      = .ok (acc + (n <<< shift), pre.size + (natBytes n).length) := by
  induction n using Nat.strong_induction_on with
  | _ n ih =>
    intro pre suf acc shift
    unfold natBytes
    split
    · rename_i hn
      have hsz : pre.size < (pre ++ [n.toUInt8].toByteArray ++ suf).size := by
        rw [size_append_mid, List.size_toByteArray, List.length_singleton]; omega
      rw [getNat_step _ _ _ _ hsz]
      have hb : (pre ++ [n.toUInt8].toByteArray ++ suf)[pre.size]'hsz = n.toUInt8 := by
        have := getElem_append_mid pre [n.toUInt8].toByteArray suf 0 (by simp) (by simpa using hsz)
        simpa using this
      rw [hb, toNat_toUInt8 n (by omega), if_pos hn, List.length_singleton]
    · rename_i hn
      have hlt : n / 128 < n := Nat.div_lt_self (by omega) (by omega)
      have hbuf : pre ++ ((n % 128 + 128).toUInt8 :: natBytes (n / 128)).toByteArray ++ suf
          = (pre ++ [(n % 128 + 128).toUInt8].toByteArray) ++ (natBytes (n / 128)).toByteArray ++ suf := by
        have : ((n % 128 + 128).toUInt8 :: natBytes (n / 128)).toByteArray
            = [(n % 128 + 128).toUInt8].toByteArray ++ (natBytes (n / 128)).toByteArray := by
          rw [← List.toByteArray_append]; rfl
        rw [this]; simp only [ByteArray.append_assoc]
      rw [hbuf]
      have hpre : (pre ++ [(n % 128 + 128).toUInt8].toByteArray).size = pre.size + 1 := by
        rw [ByteArray.size_append, List.size_toByteArray, List.length_singleton]
      have hsz : pre.size < ((pre ++ [(n % 128 + 128).toUInt8].toByteArray) ++ (natBytes (n / 128)).toByteArray ++ suf).size := by
        rw [size_append_mid, hpre]; omega
      rw [getNat_step _ _ _ _ hsz]
      have hb : ((pre ++ [(n % 128 + 128).toUInt8].toByteArray) ++ (natBytes (n / 128)).toByteArray ++ suf)[pre.size]'hsz
          = (n % 128 + 128).toUInt8 := by
        rw [ByteArray.getElem_append_left (by rw [ByteArray.size_append, hpre]; omega),
          ByteArray.getElem_append_left (by rw [hpre]; omega),
          ByteArray.getElem_append_right (Nat.le_refl _)]
        simp
      rw [hb, toNat_toUInt8 _ (by omega), if_neg (by omega), ← hpre, ih (n / 128) hlt, hpre, shift_split,
        List.length_cons]
      congr 2
      omega

/-! ### Writing straight into a buffer (D92)

`encode` builds a value's bytes as a fresh array per field and `++` copies them together;
framing a message copied its payload four times and made ~10 small allocations
(`natBytes` lists, their arrays, the appends). `pushNat` writes a natural's LEB128 bytes
onto an existing buffer, `natLen` is its length without building it; `pushNat_eq` and
`natLen_eq` are what the instances' `put`s and a direct framing loop (`frameMsgsImpl`) are
proved against. -/

/-- A natural's LEB128 bytes pushed onto `b`. -/
def pushNat (b : ByteArray) (n : Nat) : ByteArray :=
  if h : n < 128 then b.push n.toUInt8
  else pushNat (b.push (n % 128 + 128).toUInt8) (n / 128)
termination_by n
decreasing_by omega

theorem pushNat_eq (n : Nat) : ∀ (b : ByteArray), pushNat b n = b ++ (natBytes n).toByteArray := by
  induction n using Nat.strong_induction_on with
  | _ n ih =>
    intro b
    unfold pushNat natBytes
    split
    · rw [ByteArray.append_toByteArray_singleton]
    · rename_i hn
      rw [ih (n / 128) (Nat.div_lt_self (by omega) (by omega)),
        show ((n % 128 + 128).toUInt8 :: natBytes (n / 128)).toByteArray
          = [(n % 128 + 128).toUInt8].toByteArray ++ (natBytes (n / 128)).toByteArray from
          by rw [← List.toByteArray_append]; rfl,
        ← ByteArray.append_assoc, ByteArray.append_toByteArray_singleton]

/-- The number of LEB128 bytes of a natural. -/
def natLen (n : Nat) : Nat :=
  if h : n < 128 then 1 else natLen (n / 128) + 1
termination_by n
decreasing_by omega

theorem natLen_eq (n : Nat) : natLen n = (natBytes n).length := by
  induction n using Nat.strong_induction_on with
  | _ n ih =>
    unfold natLen natBytes
    split
    · rfl
    · rw [List.length_cons, ih (n / 128) (Nat.div_lt_self (by omega) (by omega))]

instance : Wire Nat where
  encode n := (natBytes n).toByteArray
  get b i := getNat b i
  get_encode n pre suf := by
    show getNat (pre ++ (natBytes n).toByteArray ++ suf) pre.size 0 0 = _
    rw [getNat_natBytes, Nat.zero_add, Nat.shiftLeft_zero, List.size_toByteArray]
  put n b := pushNat b n
  put_eq n b := pushNat_eq n b

theorem size_encode_nat (n : Nat) : (encode n).size = natLen n := by
  show (natBytes n).toByteArray.size = _
  rw [List.size_toByteArray, natLen_eq]

/-- The natural's law, in `getNat`'s own terms. -/
theorem getNat_encode (n : Nat) (pre suf : ByteArray) :
    getNat (pre ++ encode n ++ suf) pre.size = .ok (n, pre.size + (encode n).size) := get_encode n pre suf

/-! ### The containers -/

/-- Reading a value out of a buffer that continues after it: the law with the
continuation folded into the suffix — `pre ++ (encode x ++ rest) ++ suf`. -/
theorem get_encode_cont {α : Type} [Wire α] (x : α) (pre rest suf : ByteArray) :
    get (pre ++ (encode x ++ rest) ++ suf) pre.size = .ok (x, pre.size + (encode x).size) := by
  have h := get_encode x pre (rest ++ suf)
  simp only [ByteArray.append_assoc] at h ⊢
  exact h

theorem getNat_encode_cont (n : Nat) (pre rest suf : ByteArray) :
    getNat (pre ++ (encode n ++ rest) ++ suf) pre.size = .ok (n, pre.size + (encode n).size) :=
  get_encode_cont n pre rest suf

/-- The offset after a value is the size of the buffer up to it. -/
theorem size_after {α : Type} [Wire α] (x : α) (pre : ByteArray) :
    pre.size + (encode x).size = (pre ++ encode x).size := (ByteArray.size_append).symm

instance : Wire Unit where
  encode _ := .empty
  get _ i := .ok ((), i)
  get_encode _ pre _ := by simp

/-- A tag byte. -/
def getTag (b : ByteArray) (i : Nat) : Except String (Nat × Nat) :=
  if h : i < b.size then .ok ((b[i]'h).toNat, i + 1) else .error "wire: truncated tag"

def tagBytes (t : Nat) : ByteArray := [t.toUInt8].toByteArray

theorem size_tagBytes (t : Nat) : (tagBytes t).size = 1 := by simp [tagBytes]

theorem push_tag (b : ByteArray) (t : Nat) : b.push t.toUInt8 = b ++ tagBytes t :=
  (ByteArray.append_toByteArray_singleton).symm

theorem getTag_tagBytes (t : Nat) (ht : t < 256) (pre suf : ByteArray) :
    getTag (pre ++ tagBytes t ++ suf) pre.size = .ok (t, pre.size + 1) := by
  have hsz : pre.size < (pre ++ tagBytes t ++ suf).size := by
    rw [size_append_mid, size_tagBytes]; omega
  unfold getTag
  rw [dif_pos hsz]
  have hb : (pre ++ tagBytes t ++ suf)[pre.size]'hsz = t.toUInt8 := by
    have := getElem_append_mid pre (tagBytes t) suf 0 (by simp [tagBytes]) (by simpa using hsz)
    simpa [tagBytes] using this
  rw [hb, toNat_toUInt8 t ht]

/-- A tagged value: the tag then the value, read back. -/
theorem getTag_tagBytes_cont (t : Nat) (ht : t < 256) (pre rest suf : ByteArray) :
    getTag (pre ++ (tagBytes t ++ rest) ++ suf) pre.size = .ok (t, pre.size + 1) := by
  have h := getTag_tagBytes t ht pre (rest ++ suf)
  simp only [ByteArray.append_assoc] at h ⊢
  exact h

theorem size_tag_add (t : Nat) (pre : ByteArray) : pre.size + 1 = (pre ++ tagBytes t).size := by
  rw [ByteArray.size_append, size_tagBytes]

def getBool (b : ByteArray) (i : Nat) : Except String (Bool × Nat) := do
  let (t, i) ← getTag b i
  match t with
  | 0 => .ok (false, i)
  | 1 => .ok (true, i)
  | t => .error s!"wire: bad Bool tag {t}"

instance : Wire Bool where
  encode x := tagBytes (if x then 1 else 0)
  get := getBool
  get_encode x pre suf := by
    unfold getBool
    cases x
    · rw [if_neg Bool.false_ne_true, getTag_tagBytes 0 (by omega) pre suf, size_tagBytes]; rfl
    · rw [if_pos rfl, getTag_tagBytes 1 (by omega) pre suf, size_tagBytes]; rfl
  put x b := b.push (if x then 1 else 0)
  put_eq x b := by cases x <;> exact push_tag b _

/-- Zigzag: `0, -1, 1, -2, …` as `0, 1, 2, 3, …`. -/
def zigzag (x : Int) : Nat := if 0 ≤ x then 2 * x.toNat else 2 * (-x).toNat - 1
def unzigzag (n : Nat) : Int := if n % 2 = 0 then Int.ofNat (n / 2) else -(Int.ofNat ((n + 1) / 2))

theorem unzigzag_zigzag (x : Int) : unzigzag (zigzag x) = x := by
  unfold zigzag unzigzag
  split <;> simp only [Int.ofNat_eq_natCast] <;> split <;> omega

instance : Wire Int := ofEquiv zigzag unzigzag unzigzag_zigzag

def getFin (n : Nat) (b : ByteArray) (i : Nat) : Except String (Fin n × Nat) := do
  let (v, i) ← getNat b i
  if h : v < n then .ok (⟨v, h⟩, i) else .error s!"wire: Fin {n}: {v} out of range"

instance {n : Nat} : Wire (Fin n) where
  encode x := encode x.val
  get := getFin n
  get_encode x pre suf := by
    unfold getFin
    rw [getNat_encode]
    simp [bind, Except.bind, x.isLt]
  put x b := pushNat b x.val
  put_eq x b := pushNat_eq x.val b

def getProd {α β : Type} [Wire α] [Wire β] (b : ByteArray) (i : Nat) : Except String ((α × β) × Nat) := do
  let (a, i) ← get (α := α) b i
  let (c, i) ← get (α := β) b i
  .ok ((a, c), i)

instance {α β : Type} [Wire α] [Wire β] : Wire (α × β) where
  encode x := encode x.1 ++ encode x.2
  get := getProd
  get_encode x pre suf := by
    unfold getProd
    rw [get_encode_cont]
    simp only [bind, Except.bind]
    have hre : pre ++ (encode x.1 ++ encode x.2) ++ suf = pre ++ encode x.1 ++ encode x.2 ++ suf := by
      simp only [ByteArray.append_assoc]
    rw [size_after]
    simp only [hre]
    rw [get_encode x.2 (pre ++ encode x.1) suf]
    simp only [ByteArray.size_append, Nat.add_assoc]
  put x b := put x.2 (put x.1 b)
  put_eq x b := by rw [put_eq, put_eq, ByteArray.append_assoc]

def getOption {α : Type} [Wire α] (b : ByteArray) (i : Nat) : Except String (Option α × Nat) := do
  let (t, i) ← getTag b i
  match t with
  | 0 => .ok (none, i)
  | 1 => do let (a, i) ← get (α := α) b i; .ok (some a, i)
  | t => .error s!"wire: bad Option tag {t}"

instance {α : Type} [Wire α] : Wire (Option α) where
  encode x := match x with
    | none => tagBytes 0
    | some a => tagBytes 1 ++ encode a
  get := getOption
  put x b := match x with
    | none => b.push 0
    | some a => put a (b.push 1)
  put_eq x b := by
    cases x with
    | none => exact push_tag b 0
    | some a =>
      show put a (b.push (Nat.toUInt8 1)) = b ++ (tagBytes 1 ++ encode a)
      rw [put_eq, push_tag, ByteArray.append_assoc]
  get_encode x pre suf := by
    unfold getOption
    cases x with
    | none => rw [getTag_tagBytes 0 (by omega) pre suf, size_tagBytes]; rfl
    | some a =>
      rw [getTag_tagBytes_cont 1 (by omega)]
      simp only [bind, Except.bind]
      have hre : pre ++ (tagBytes 1 ++ encode a) ++ suf = pre ++ tagBytes 1 ++ encode a ++ suf := by
        simp only [ByteArray.append_assoc]
      rw [size_tag_add 1]
      simp only [hre]
      rw [get_encode a (pre ++ tagBytes 1) suf]
      simp only [ByteArray.size_append, size_tagBytes, Nat.add_assoc]

def getExcept {ε α : Type} [Wire ε] [Wire α] (b : ByteArray) (i : Nat) : Except String (Except ε α × Nat) := do
  let (t, i) ← getTag b i
  match t with
  | 0 => do let (e, i) ← get (α := ε) b i; .ok (.error e, i)
  | 1 => do let (a, i) ← get (α := α) b i; .ok (.ok a, i)
  | t => .error s!"wire: bad Except tag {t}"

instance {ε α : Type} [Wire ε] [Wire α] : Wire (Except ε α) where
  encode x := match x with
    | .error e => tagBytes 0 ++ encode e
    | .ok a => tagBytes 1 ++ encode a
  get := getExcept
  put x b := match x with
    | .error e => put e (b.push 0)
    | .ok a => put a (b.push 1)
  put_eq x b := by
    cases x with
    | error e =>
      show put e (b.push (Nat.toUInt8 0)) = b ++ (tagBytes 0 ++ encode e)
      rw [put_eq, push_tag, ByteArray.append_assoc]
    | ok a =>
      show put a (b.push (Nat.toUInt8 1)) = b ++ (tagBytes 1 ++ encode a)
      rw [put_eq, push_tag, ByteArray.append_assoc]
  get_encode x pre suf := by
    unfold getExcept
    cases x with
    | error e =>
      rw [getTag_tagBytes_cont 0 (by omega)]
      simp only [bind, Except.bind]
      have hre : pre ++ (tagBytes 0 ++ encode e) ++ suf = pre ++ tagBytes 0 ++ encode e ++ suf := by
        simp only [ByteArray.append_assoc]
      rw [size_tag_add 0]
      simp only [hre]
      rw [get_encode e (pre ++ tagBytes 0) suf]
      simp only [ByteArray.size_append, size_tagBytes, Nat.add_assoc]
    | ok a =>
      rw [getTag_tagBytes_cont 1 (by omega)]
      simp only [bind, Except.bind]
      have hre : pre ++ (tagBytes 1 ++ encode a) ++ suf = pre ++ tagBytes 1 ++ encode a ++ suf := by
        simp only [ByteArray.append_assoc]
      rw [size_tag_add 1]
      simp only [hre]
      rw [get_encode a (pre ++ tagBytes 1) suf]
      simp only [ByteArray.size_append, size_tagBytes, Nat.add_assoc]

def getList {α : Type} [Wire α] (b : ByteArray) : Nat → Nat → List α → Except String (List α × Nat)
  | 0, i, acc => .ok (acc.reverse, i)
  | k + 1, i, acc => do
    let (a, i) ← get (α := α) b i
    getList b k i (a :: acc)

/-- The items' bytes, in order. -/
def listBytes {α : Type} [Wire α] : List α → ByteArray
  | [] => .empty
  | x :: xs => encode x ++ listBytes xs

/-- The items read back, one by one. -/
theorem getList_listBytes {α : Type} [Wire α] : ∀ (xs : List α) (pre suf : ByteArray) (acc : List α),
    getList (pre ++ listBytes xs ++ suf) xs.length pre.size acc = .ok (acc.reverse ++ xs, pre.size + (listBytes xs).size)
  | [], pre, suf, acc => by simp [getList, listBytes]
  | x :: xs, pre, suf, acc => by
    simp only [List.length_cons, getList, listBytes]
    rw [get_encode_cont]
    simp only [bind, Except.bind]
    have hre : pre ++ (encode x ++ listBytes xs) ++ suf = pre ++ encode x ++ listBytes xs ++ suf := by
      simp only [ByteArray.append_assoc]
    rw [size_after]
    simp only [hre]
    rw [getList_listBytes xs (pre ++ encode x) suf]
    simp [ByteArray.size_append, Nat.add_assoc]

def getListW {α : Type} [Wire α] (b : ByteArray) (i : Nat) : Except String (List α × Nat) := do
  let (k, i) ← getNat b i
  getList b k i []

/-- The items written one after another onto a buffer. -/
theorem foldl_put_listBytes {α : Type} [Wire α] : ∀ (xs : List α) (b : ByteArray),
    xs.foldl (fun b x => put x b) b = b ++ listBytes xs
  | [], b => by simp [listBytes]
  | x :: xs, b => by
    rw [List.foldl_cons, foldl_put_listBytes xs, put_eq, listBytes, ByteArray.append_assoc]

instance {α : Type} [Wire α] : Wire (List α) where
  encode xs := encode xs.length ++ listBytes xs
  get := getListW
  put xs b := xs.foldl (fun b x => put x b) (pushNat b xs.length)
  put_eq xs b := by
    show xs.foldl (fun b x => put x b) (pushNat b xs.length) = b ++ (encode xs.length ++ listBytes xs)
    rw [foldl_put_listBytes, pushNat_eq, ByteArray.append_assoc]; rfl
  get_encode xs pre suf := by
    unfold getListW
    rw [getNat_encode_cont]
    simp only [bind, Except.bind]
    have hre : pre ++ (encode xs.length ++ listBytes xs) ++ suf = pre ++ encode xs.length ++ listBytes xs ++ suf := by
      simp only [ByteArray.append_assoc]
    rw [size_after]
    simp only [hre]
    rw [getList_listBytes xs (pre ++ encode xs.length) suf]
    simp [ByteArray.size_append, Nat.add_assoc]

def getBytes (b : ByteArray) (i : Nat) : Except String (ByteArray × Nat) := do
  let (k, i) ← getNat b i
  if i + k ≤ b.size then .ok (b.extract i (i + k), i + k) else .error "wire: truncated bytes"

/-- Raw bytes, length-prefixed. -/
instance : Wire ByteArray where
  encode x := encode x.size ++ x
  get := getBytes
  put x b := pushNat b x.size ++ x
  put_eq x b := by
    show pushNat b x.size ++ x = b ++ (encode x.size ++ x)
    rw [pushNat_eq, ByteArray.append_assoc]; rfl
  get_encode x pre suf := by
    unfold getBytes
    rw [getNat_encode_cont]
    simp only [bind, Except.bind]
    have hre : pre ++ (encode x.size ++ x) ++ suf = pre ++ encode x.size ++ x ++ suf := by
      simp only [ByteArray.append_assoc]
    rw [size_after]
    simp only [hre]
    have hle : (pre ++ encode x.size).size + x.size ≤ (pre ++ encode x.size ++ x ++ suf).size := by
      simp only [ByteArray.size_append]; omega
    rw [if_pos hle, ByteArray.extract_append, ByteArray.extract_append_eq_right rfl rfl]
    have h0 : (pre ++ encode x.size).size + x.size - (pre ++ encode x.size ++ x).size = 0 := by
      rw [ByteArray.size_append (a := pre ++ encode x.size)]; omega
    have h0' : (pre ++ encode x.size).size - (pre ++ encode x.size ++ x).size = 0 := by
      rw [ByteArray.size_append (a := pre ++ encode x.size)]; omega
    rw [h0, h0', ByteArray.extract_same, ByteArray.append_empty]
    simp only [ByteArray.size_append, Nat.add_assoc]

def getString (b : ByteArray) (i : Nat) : Except String (String × Nat) := do
  let (bs, i) ← getBytes b i
  match String.fromUTF8? bs with
  | some s => .ok (s, i)
  | none => .error "wire: bad UTF-8"

theorem fromUTF8?_toByteArray (s : String) : String.fromUTF8? s.toByteArray = some s := by
  simp only [String.fromUTF8?]
  rw [dif_pos s.isValidUTF8]
  rfl

instance : Wire String where
  encode s := encode s.toUTF8
  get := getString
  put s b := put s.toUTF8 b
  put_eq s b := put_eq s.toUTF8 b
  get_encode s pre suf := by
    unfold getString
    rw [show getBytes (pre ++ encode s.toUTF8 ++ suf) pre.size = get (pre ++ encode s.toUTF8 ++ suf) pre.size from rfl,
      get_encode s.toUTF8 pre suf]
    simp [bind, Except.bind, fromUTF8?_toByteArray]

/-! ### Frames — a byte stream cut into values (D90)

A connection is a byte stream; a frame is one value's bytes, length-prefixed (`frame x
= encode (encode x : ByteArray)`: the size as a varint, then the bytes). `frames` reads
every complete frame off a buffer and returns the incomplete tail; a receiver keeps the
tail and prepends it to its next read. The laws below are what make a run at the bytes
the run at the messages (`Dag.iterateBytes`, `WireTrace`): cutting the stream of frames
anywhere, `frames` yields exactly the frames that are complete and keeps the rest
(`frames_prefix_frameAll`). -/

def frame {α : Type} [Wire α] (x : α) : ByteArray := encode (encode x)

/-- The bytes of a list of values, framed one after another. -/
def frameAll {α : Type} [Wire α] : List α → ByteArray
  | [] => .empty
  | x :: xs => frame x ++ frameAll xs

theorem frameAll_append {α : Type} [Wire α] (xs ys : List α) : frameAll (xs ++ ys) = frameAll xs ++ frameAll ys := by
  induction xs with
  | nil => simp [frameAll]
  | cons x xs ih => simp [frameAll, ih, ByteArray.append_assoc]

/-- `frameAll` compiled: the frames appended onto one growing buffer (the definition's
right-nested `frame x ++ frameAll xs` copies the whole tail at every frame — quadratic in a
bucket's size; the leader's buckets hold ~50 messages, D92). -/
def frameAllImpl {α : Type} [Wire α] (xs : List α) : ByteArray :=
  xs.foldl (fun acc x => acc ++ frame x) .empty

theorem frameAllImpl_foldl {α : Type} [Wire α] (xs : List α) (acc : ByteArray) :
    xs.foldl (fun acc x => acc ++ frame x) acc = acc ++ frameAll xs := by
  induction xs generalizing acc with
  | nil => simp [frameAll]
  | cons x xs ih => rw [List.foldl_cons, ih, frameAll, ByteArray.append_assoc]

@[csimp] theorem frameAll_eq_impl : @frameAll = @frameAllImpl := by
  funext α _ xs
  rw [frameAllImpl, frameAllImpl_foldl, ByteArray.empty_append]

theorem size_frameAll_cons {α : Type} [Wire α] (x : α) (xs : List α) :
    (frameAll (x :: xs)).size = (frame x).size + (frameAll xs).size := ByteArray.size_append

/-- The complete frames from offset `i` on (decoded, in order, after `acc`) and the tail
from the first incomplete one. -/
def framesFrom {α : Type} [Wire α] (buf : ByteArray) (i : Nat) (acc : Array (Except String α)) :
    Array (Except String α) × ByteArray :=
  match getBytes buf i with
  | .ok (body, j) =>
    if hj : i < j ∧ j ≤ buf.size then framesFrom buf j (acc.push (decode body))
    else (acc, buf.extract i buf.size)
  | .error _ => (acc, buf.extract i buf.size)
termination_by buf.size - i
decreasing_by omega

/-- The complete frames at the front of a buffer (decoded), and the incomplete tail. -/
def frames {α : Type} [Wire α] (buf : ByteArray) : Array (Except String α) × ByteArray :=
  framesFrom buf 0 #[]

/-! #### Reading is translation-invariant and advances -/

theorem extract_zero_of_le (a : ByteArray) {j : Nat} (h : a.size ≤ j) : a.extract 0 j = a := by
  ext1
  rw [ByteArray.data_extract, Array.extract_eq_self_of_le]
  rw [ByteArray.size_data]; exact h

theorem extract_toByteArray (l : List UInt8) (b : Nat) : l.toByteArray.extract 0 b = (l.take b).toByteArray := by
  ext1
  simp

theorem getNat_shift (pre b : ByteArray) (i acc shift : Nat) :
    getNat (pre ++ b) (pre.size + i) acc shift = (getNat b i acc shift).map fun r => (r.1, pre.size + r.2) := by
  by_cases h : i < b.size
  · have h' : pre.size + i < (pre ++ b).size := by rw [ByteArray.size_append]; omega
    rw [getNat_step _ _ _ _ h', getNat_step b i acc shift h]
    have hb : (pre ++ b)[pre.size + i]'h' = b[i]'h := by
      rw [ByteArray.getElem_append_right (Nat.le_add_right _ _)]; simp
    rw [hb]
    split
    · rfl
    · rw [Nat.add_assoc]; exact getNat_shift pre b (i + 1) _ _
  · have h' : ¬ pre.size + i < (pre ++ b).size := by rw [ByteArray.size_append]; omega
    rw [getNat, dif_neg h', getNat, dif_neg h]; rfl
termination_by b.size - i

theorem getNat_ok_bounds (b : ByteArray) (i acc shift : Nat) {n j : Nat}
    (h : getNat b i acc shift = .ok (n, j)) : i < j ∧ j ≤ b.size := by
  by_cases hi : i < b.size
  · rw [getNat_step _ _ _ _ hi] at h
    split at h
    · cases h; omega
    · have := getNat_ok_bounds b (i + 1) _ _ h; omega
  · rw [getNat, dif_neg hi] at h; cases h
termination_by b.size - i

theorem getBytes_shift (pre b : ByteArray) (i : Nat) :
    getBytes (pre ++ b) (pre.size + i) = (getBytes b i).map fun r => (r.1, pre.size + r.2) := by
  unfold getBytes
  rw [getNat_shift]
  rcases getNat b i with e | ⟨k, i'⟩
  · rfl
  · simp only [Except.map, bind, Except.bind, ByteArray.size_append]
    by_cases hk : i' + k ≤ b.size
    · rw [if_pos hk, if_pos (by omega), Nat.add_assoc, ByteArray.extract_append_size_add]
    · rw [if_neg hk, if_neg (by omega)]

theorem getBytes_ok_bounds (b : ByteArray) (i : Nat) {body : ByteArray} {j : Nat}
    (h : getBytes b i = .ok (body, j)) : i < j ∧ j ≤ b.size := by
  unfold getBytes at h
  rcases hg : getNat b i with e | ⟨k, i'⟩
  · rw [hg] at h; cases h
  · rw [hg] at h
    simp only [bind, Except.bind] at h
    split at h
    · cases h
      have := getNat_ok_bounds b i 0 0 hg
      omega
    · cases h

theorem framesFrom_shift {α : Type} [Wire α] (pre b : ByteArray) (i : Nat) (acc : Array (Except String α)) :
    framesFrom (pre ++ b) (pre.size + i) acc = framesFrom b i acc := by
  rw [framesFrom, framesFrom, getBytes_shift]
  rcases hg : getBytes b i with e | ⟨body, j⟩
  · simp only [Except.map]
    rw [ByteArray.size_append, ByteArray.extract_append_size_add]
  · simp only [Except.map]
    have hs : (pre ++ b).size = pre.size + b.size := ByteArray.size_append
    by_cases hj : i < j ∧ j ≤ b.size
    · rw [dif_pos (by omega), dif_pos hj]
      exact framesFrom_shift pre b j _
    · rw [dif_neg (by omega), dif_neg hj, hs, ByteArray.extract_append_size_add]
termination_by b.size - i

/-- A natural's bytes are never empty. -/
theorem size_encode_nat_pos (n : Nat) : 0 < (encode n).size := by
  show 0 < (natBytes n).toByteArray.size
  rw [List.size_toByteArray]
  unfold natBytes
  split <;> simp

theorem size_frame_pos {α : Type} [Wire α] (x : α) : 0 < (frame x).size := by
  show 0 < (encode (encode x).size ++ encode x).size
  rw [ByteArray.size_append]
  have := size_encode_nat_pos (encode x).size
  omega

/-- **A complete frame at the front is read and skipped.** -/
theorem framesFrom_frame_append {α : Type} [Wire α] (x : α) (rest : ByteArray) (acc : Array (Except String α)) :
    framesFrom (frame x ++ rest) 0 acc = framesFrom rest 0 (acc.push (.ok x)) := by
  rw [framesFrom]
  have h := get_encode (encode x) ByteArray.empty rest
  rw [ByteArray.empty_append, ByteArray.size_empty, Nat.zero_add] at h
  change getBytes (frame x ++ rest) 0 = _ at h
  rw [h]
  simp only
  rw [dif_pos ⟨by show 0 < (frame x).size; exact size_frame_pos x,
    by show (frame x).size ≤ (frame x ++ rest).size; rw [ByteArray.size_append]; omega⟩]
  have hsh := framesFrom_shift (frame x) rest 0 (acc.push (decode (encode x)))
  rw [Nat.add_zero] at hsh
  rw [show (encode (encode x)).size = (frame x).size from rfl, hsh, decode_encode]

/-! #### An incomplete frame is kept -/

/-- A strict prefix of a natural's bytes does not read. -/
theorem getNat_natBytes_trunc (n : Nat) : ∀ (j acc shift : Nat), j < (natBytes n).length →
    ∃ e, getNat ((natBytes n).take j).toByteArray 0 acc shift = .error e := by
  induction n using Nat.strong_induction_on with
  | _ n ih =>
    intro j acc shift hj
    rw [natBytes] at hj ⊢
    by_cases hn : n < 128
    · rw [dif_pos hn] at hj ⊢
      rw [List.length_singleton] at hj
      have : j = 0 := by omega
      subst this
      exact ⟨_, by rw [getNat, dif_neg (by simp)]⟩
    · rw [dif_neg hn] at hj ⊢
      rcases j with _ | j
      · exact ⟨_, by rw [getNat, dif_neg (by simp)]⟩
      · rw [List.length_cons] at hj
        rw [List.take_succ_cons]
        have hlt : n / 128 < n := Nat.div_lt_self (by omega) (by omega)
        obtain ⟨e, he⟩ := ih (n / 128) hlt j (acc + ((n % 128 + 128 - 128) <<< shift)) (shift + 7) (by omega)
        have hbuf : ((n % 128 + 128).toUInt8 :: (natBytes (n / 128)).take j).toByteArray
            = [(n % 128 + 128).toUInt8].toByteArray ++ ((natBytes (n / 128)).take j).toByteArray := by
          rw [← List.toByteArray_append]; rfl
        have hsz : 0 < (((n % 128 + 128).toUInt8 :: (natBytes (n / 128)).take j).toByteArray).size := by
          rw [List.size_toByteArray]; simp
        refine ⟨e, ?_⟩
        rw [getNat_step _ _ _ _ hsz]
        have hb : (((n % 128 + 128).toUInt8 :: (natBytes (n / 128)).take j).toByteArray)[0]'hsz
            = (n % 128 + 128).toUInt8 := by simp
        rw [hb, toNat_toUInt8 _ (by omega), if_neg (by omega), hbuf]
        have h1 : ([(n % 128 + 128).toUInt8].toByteArray).size = 1 := by simp
        rw [show (0 + 1 : Nat) = ([(n % 128 + 128).toUInt8].toByteArray).size + 0 from by rw [h1], getNat_shift, he]
        rfl

/-- A strict prefix of length-prefixed bytes does not read. -/
theorem getBytes_trunc (body : ByteArray) (b : Nat) (hb : b < (encode body).size) :
    ∃ e, getBytes ((encode body).extract 0 b) 0 = .error e := by
  change ∃ e, getBytes ((encode body.size ++ body).extract 0 b) 0 = .error e
  have hL : (encode body.size).size = (natBytes body.size).length := List.size_toByteArray
  by_cases hbL : b < (natBytes body.size).length
  · have hex : (encode body.size ++ body).extract 0 b = ((natBytes body.size).take b).toByteArray := by
      rw [ByteArray.extract_append]
      have : body.extract (0 - (encode body.size).size) (b - (encode body.size).size) = ByteArray.empty := by
        rw [ByteArray.extract_eq_empty_iff]; omega
      rw [this, ByteArray.append_empty]
      exact extract_toByteArray _ _
    obtain ⟨e, he⟩ := getNat_natBytes_trunc body.size b 0 0 hbL
    refine ⟨e, ?_⟩
    unfold getBytes
    rw [hex, he]
    rfl
  · have hex : (encode body.size ++ body).extract 0 b = encode body.size ++ body.extract 0 (b - (encode body.size).size) := by
      rw [ByteArray.extract_append, Nat.zero_sub]
      congr 1
      exact extract_zero_of_le _ (by omega)
    rw [hex]
    unfold getBytes
    have h := getNat_natBytes body.size ByteArray.empty (body.extract 0 (b - (encode body.size).size)) 0 0
    rw [ByteArray.empty_append, ByteArray.size_empty] at h
    change getNat (encode body.size ++ body.extract 0 (b - (encode body.size).size)) 0 0 0 = _ at h
    rw [h]
    simp only [bind, Except.bind, Nat.zero_add, Nat.shiftLeft_zero, ByteArray.size_append, ByteArray.size_extract]
    have hb' : b < (natBytes body.size).length + body.size := by
      have := hb
      change b < (encode body.size ++ body).size at this
      rw [ByteArray.size_append, hL] at this
      exact this
    rw [if_neg]
    · exact ⟨_, rfl⟩
    · rw [hL]; omega

/-- **An incomplete frame is kept whole** (`frames` of a strict prefix of a frame reads nothing). -/
theorem framesFrom_partial {α : Type} [Wire α] (x : α) (b : Nat) (hb : b < (frame x).size) (acc : Array (Except String α)) :
    framesFrom ((frame x).extract 0 b) 0 acc = (acc, (frame x).extract 0 b) := by
  obtain ⟨e, he⟩ := getBytes_trunc (encode x) b hb
  have he' : getBytes ((frame x).extract 0 b) 0 = .error e := he
  rw [framesFrom, he']
  simp only
  rw [ByteArray.extract_zero_size]

/-! #### Any cut of a stream of frames -/

/-- How many of the framed values fit in the first `b` bytes, and how many bytes of the
next one. -/
def splitFrames {α : Type} [Wire α] : List α → Nat → Nat × Nat
  | [], b => (0, b)
  | x :: xs, b => if (frame x).size ≤ b then let r := splitFrames xs (b - (frame x).size); (r.1 + 1, r.2) else (0, b)

/-- **Cutting a stream of frames anywhere**: `frames` of its first `b` bytes is the frames
that fit, decoded, and the bytes of the one that does not. -/
theorem framesFrom_prefix_frameAll {α : Type} [Wire α] : ∀ (xs : List α) (b : Nat), b ≤ (frameAll xs).size →
    ∀ (acc : Array (Except String α)),
    framesFrom ((frameAll xs).extract 0 b) 0 acc
      = (acc ++ ((xs.take (splitFrames xs b).1).map .ok).toArray,
         (frameAll (xs.drop (splitFrames xs b).1)).extract 0 (splitFrames xs b).2)
  | [], b, hb, acc => by
    simp only [frameAll, ByteArray.size_empty] at hb
    have : b = 0 := by omega
    subst this
    have hg : getBytes (ByteArray.empty.extract 0 0) 0 = .error "wire: truncated natural" := by
      unfold getBytes; rw [getNat, dif_neg (by simp)]; rfl
    simp only [frameAll]
    rw [framesFrom, hg]
    simp [splitFrames, frameAll]
  | x :: xs, b, hb, acc => by
    simp only [frameAll, splitFrames]
    split
    · rename_i hle
      have hex : (frame x ++ frameAll xs).extract 0 b = frame x ++ (frameAll xs).extract 0 (b - (frame x).size) := by
        rw [ByteArray.extract_append, Nat.zero_sub]
        congr 1
        exact extract_zero_of_le _ hle
      rw [hex, framesFrom_frame_append]
      rw [size_frameAll_cons] at hb
      rw [framesFrom_prefix_frameAll xs (b - (frame x).size) (by omega)]
      refine Prod.ext ?_ rfl
      apply Array.ext'
      simp
    · rename_i hlt
      have hex : (frame x ++ frameAll xs).extract 0 b = (frame x).extract 0 b := by
        rw [ByteArray.extract_append]
        have : (frameAll xs).extract (0 - (frame x).size) (b - (frame x).size) = ByteArray.empty := by
          rw [ByteArray.extract_eq_empty_iff]; omega
        rw [this, ByteArray.append_empty]
      rw [List.take_zero, List.drop_zero, List.map_nil]
      show _ = (acc ++ #[], (frame x ++ frameAll xs).extract 0 b)
      rw [Array.append_empty]
      rw [hex, framesFrom_partial x b (by omega)]


/-! #### `splitFrames`: the count fits, adds up, and is stable under growth -/

theorem splitFrames_fst_le_length {α : Type} [Wire α] : ∀ (xs : List α) (b : Nat), (splitFrames xs b).1 ≤ xs.length
  | [], _ => Nat.le_refl _
  | x :: xs, b => by
    simp only [splitFrames, List.length_cons]
    split
    · exact Nat.succ_le_succ (splitFrames_fst_le_length xs _)
    · exact Nat.zero_le _

/-- The bytes of the frames that fit plus the leftover is the budget. -/
theorem splitFrames_spec {α : Type} [Wire α] : ∀ (xs : List α) (b : Nat), b ≤ (frameAll xs).size →
    (frameAll (xs.take (splitFrames xs b).1)).size + (splitFrames xs b).2 = b
  | [], b, hb => by
    simp only [frameAll, ByteArray.size_empty] at hb
    simp [splitFrames, frameAll] <;> omega
  | x :: xs, b, hb => by
    simp only [splitFrames]
    split
    · rename_i hle
      rw [size_frameAll_cons] at hb
      have ih := splitFrames_spec xs (b - (frame x).size) (by omega)
      simp only [List.take_succ_cons, size_frameAll_cons]
      omega
    · simp [frameAll]

/-- The leftover is within the frames that did not fit. -/
theorem splitFrames_snd_le {α : Type} [Wire α] (xs : List α) (b : Nat) (hb : b ≤ (frameAll xs).size) :
    (splitFrames xs b).2 ≤ (frameAll (xs.drop (splitFrames xs b).1)).size := by
  have h := splitFrames_spec xs b hb
  have hsz : (frameAll xs).size = (frameAll (xs.take (splitFrames xs b).1)).size
      + (frameAll (xs.drop (splitFrames xs b).1)).size := by
    rw [← ByteArray.size_append, ← frameAll_append, List.take_append_drop]
  omega

/-- More budget: the frames that fit so far, then what fits of the rest in the leftover plus
the extra. -/
theorem splitFrames_add {α : Type} [Wire α] : ∀ (xs : List α) (b δ : Nat),
    splitFrames xs (b + δ)
      = ((splitFrames xs b).1 + (splitFrames (xs.drop (splitFrames xs b).1) ((splitFrames xs b).2 + δ)).1,
         (splitFrames (xs.drop (splitFrames xs b).1) ((splitFrames xs b).2 + δ)).2)
  | [], b, δ => by simp [splitFrames]
  | x :: xs, b, δ => by
    by_cases hle : (frame x).size ≤ b
    · have h1 : splitFrames (x :: xs) b
          = ((splitFrames xs (b - (frame x).size)).1 + 1, (splitFrames xs (b - (frame x).size)).2) := by
        simp [splitFrames, hle]
      have h2 : splitFrames (x :: xs) (b + δ)
          = ((splitFrames xs (b + δ - (frame x).size)).1 + 1, (splitFrames xs (b + δ - (frame x).size)).2) := by
        simp [splitFrames, show (frame x).size ≤ b + δ by omega]
      rw [h1, h2]
      simp only [List.drop_succ_cons]
      rw [show b + δ - (frame x).size = (b - (frame x).size) + δ by omega, splitFrames_add xs (b - (frame x).size) δ]
      refine Prod.ext ?_ rfl
      show _ + _ + 1 = _ + 1 + _
      omega
    · have h1 : splitFrames (x :: xs) b = (0, b) := by simp [splitFrames, hle]
      rw [h1]
      simp

theorem splitFrames_fst_mono {α : Type} [Wire α] (xs : List α) {b b' : Nat} (h : b ≤ b') :
    (splitFrames xs b).1 ≤ (splitFrames xs b').1 := by
  rw [show b' = b + (b' - b) by omega, splitFrames_add]
  exact Nat.le_add_right _ _

/-- Within the bytes of `xs`, more frames behind do not change the split. -/
theorem splitFrames_append_of_le {α : Type} [Wire α] : ∀ (xs ys : List α) (b : Nat), b ≤ (frameAll xs).size →
    splitFrames (xs ++ ys) b = splitFrames xs b
  | [], ys, b, hb => by
    simp only [frameAll, ByteArray.size_empty] at hb
    have : b = 0 := by omega
    subst this
    cases ys with
    | nil => rfl
    | cons y ys =>
      simp only [List.nil_append, splitFrames]
      rw [if_neg]
      have := size_frame_pos y
      omega
  | x :: xs, ys, b, hb => by
    simp only [List.cons_append, splitFrames]
    split
    · rw [size_frameAll_cons] at hb
      rw [splitFrames_append_of_le xs ys _ (by omega)]
    · rfl

/-- The byte stream cut at a split: the frames before it, then the leftover of the rest. -/
theorem frameAll_extract_split {α : Type} [Wire α] (xs : List α) (b : Nat) (hb : b ≤ (frameAll xs).size) :
    (frameAll xs).extract (frameAll (xs.take (splitFrames xs b).1)).size b
      = (frameAll (xs.drop (splitFrames xs b).1)).extract 0 (splitFrames xs b).2 := by
  have h := splitFrames_spec xs b hb
  set p := (splitFrames xs b).1 with hp
  set l := (splitFrames xs b).2 with hl
  have hx : frameAll xs = frameAll (xs.take p) ++ frameAll (xs.drop p) := by
    rw [← frameAll_append, List.take_append_drop]
  rw [hx, show b = (frameAll (xs.take p)).size + l from h.symm, ByteArray.extract_append]
  have h0 : (frameAll (xs.take p)).extract (frameAll (xs.take p)).size ((frameAll (xs.take p)).size + l)
      = ByteArray.empty := by
    rw [ByteArray.extract_eq_empty_iff]; omega
  rw [h0, ByteArray.empty_append, Nat.sub_self, Nat.add_sub_cancel_left]

/-- The kept tail is unchanged by frames arriving behind the budget. -/
theorem tail_append_of_le {α : Type} [Wire α] (xs ys : List α) (b : Nat) (hb : b ≤ (frameAll xs).size) :
    (frameAll ((xs ++ ys).drop (splitFrames (xs ++ ys) b).1)).extract 0 (splitFrames (xs ++ ys) b).2
      = (frameAll (xs.drop (splitFrames xs b).1)).extract 0 (splitFrames xs b).2 := by
  rw [splitFrames_append_of_le xs ys b hb,
    List.drop_append_of_le_length (splitFrames_fst_le_length xs b), frameAll_append, ByteArray.extract_append]
  have h0 : (frameAll ys).extract (0 - (frameAll (xs.drop (splitFrames xs b).1)).size)
      ((splitFrames xs b).2 - (frameAll (xs.drop (splitFrames xs b).1)).size) = ByteArray.empty := by
    rw [ByteArray.extract_eq_empty_iff]
    have := splitFrames_snd_le xs b hb
    omega
  rw [h0, ByteArray.append_empty]

/-- Decoded frames back to their values. -/
theorem toList_filterMap_ok {α : Type} (xs : List α) :
    (((xs.map (Except.ok (ε := String))).toArray).toList.filterMap Except.toOption) = xs := by
  simp [List.filterMap_map, Function.comp_def, Except.toOption]

end Wire

/-- A payload codec: what a network node's items look like on the wire, with its
round trip. -/
structure Codec (α : Type) where
  enc : α → ByteArray
  dec : ByteArray → Except String α
  dec_enc : ∀ x, dec (enc x) = .ok x

def Codec.ofWire {α : Type} [Wire α] : Codec α :=
  ⟨Wire.putEmpty, Wire.decode, fun x => by rw [Wire.putEmpty_eq]; exact Wire.decode_encode x⟩

/-- A codec of a wire form encodes by `encode` (the specification; `putEmpty_eq`). -/
@[simp] theorem Codec.ofWire_enc {α : Type} [Wire α] (x : α) : (Codec.ofWire (α := α)).enc x = Wire.encode x :=
  Wire.putEmpty_eq x

end Hydro
