---
sidebar_position: 1
---

# Lattice Math

Lattices are a simple but powerful mathematic concept that can greatly simplify programming distributed systems.

## Lattices For Dummies

So, what is a lattice?
Lattices are conceptually very simple, so let's explain them without too much mathy language.
A lattice is some type of thing that has a very special _merge_ function.
The merge function combines two things and produces an output thing. The core feature of
lattices is that the merge function has some special properties: _associativity_, _commutativity_,
and _idempotence_ (abbreviated ACI).

Lets start with a simple example of something which turns out to **not** be a lattice: _numbers_
and _addition_. Numbers are a type of thing, and addition is a function that takes two input
numbers and produces an output number! But does it satisfy the ACI properties?

Let's start with _commutativity_. A function is commutative if it doesn't matter if you swap its
two inputs:
$$
    a + b = b + a
$$
Addition is commutative, so that property is satisfied! (In contrast, subtraction is **not**
commutative).

Next _associativity_. A function is associative if, when there are multiple calls together, it does
not matter in what order you evaluate the calls:
$$
    (a + b) + c = a + (b + c)
$$
Addition satisfies associativity! (Again, subtraction is not associative -- if you subtract a
subtraction then you're doing addition!)

Finally, _idempotence_. A function is idempotent if when you give it the same value twice it
returns that same value.
$$
    a + a \neq a
$$
Addition is **NOT** idempotent, because $a+a$ is $2a$, not just $a$. This works for $a=0$, but
not for all numbers.

Now lets look at something that _is_ a lattice. Numbers and _the `max` function_. It is
commutiative; the max of $a, b$ is the same as the max of $b, a$. It is associative; the max of
$a, b, c$ will always be the same no mater what order you look at them in. And, unlike addition,
_it is idempotent;_ the max of $a$ and $a$ is always just $a$.

The most standard, canonical example of a lattice is _sets_ with the _union_ function. It is
commutative and associative; it doesn't matter what order you union sets together, the result will
always be the same. And it is idempotent; a set unioned with itself is the same set.

### Lattice Partial Order

When learning about lattices you'll hear about some sort of "partial order" relating to them. What
is a partial order? It's like a normal order, where you can say "$a$ comes before $b$", but it is
partial because sometimes say "$a$ and $b$ are _incomparable_" instead. As it turns out, the
lattice merge function actually creates a partial order on the elements of the lattice.
Specifically, we say that whenever you merge two things, the thing you get out is always _bigger than_
each of the inputs (or it might be equal to one or both of them).

So for the number-max lattice, $\operatorname{max}(3, 6) = 6$ tells us that $3\leq 6$ and $6\leq 6$.
Ok duh. It turns out that the partial order created by the `max` merge function is naturally just
numerical order (i.e. $\leq$). Additionally, it is a total order, meaning all pairs of items are
comparable. But things get a bit more interesting with other lattices.

||
| :---: |
| ![A vertical number line starting at 1, with arrows pointing from 1 to 2, 2 to 3, etc.](../img/max-int-ord.png) |
| A visualization of the `max` total order over positive integers. |

Let's consider the set-union lattice. As it turns out, the partial order created by the union
($\cup$) function matches subset order, $\subseteq$.

$$
    \left\{ 1, 2, 3 \right\} \cup \left\{ 3, 4 \right\} = \left\{ 1, 2, 3, 4 \right\} \\
    ~ \\
    \left\{ 1, 2, 3 \right\} \subseteq \left\{ 1, 2, 3, 4 \right\} \\
    \left\{ 3, 4 \right\} \subseteq \left\{ 1, 2, 3, 4 \right\}
$$

So, both of these subsets, $\left\{ 1, 2, 3 \right\}$ and $\left\{ 3, 4 \right\}$ are _before_ $\left\{ 1, 2, 3, 4 \right\}$.
Or equivalently, $\left\{ 1, 2, 3, 4 \right\}$ is _larger_.

But what about the ordering between $\left\{ 1, 2, 3 \right\}$ and $\left\{ 3, 4 \right\}$? It
turns out these two sets are _incomparable_ under the set-union order. Neither is a subset of the
other, so neither can come first. Equivalently, going back to the merge function, there is no thing
which you could merge (union) into one of the sets in order to get the other set out.

So above, we visualized the number-max lattice partial order as a graph, albeit a very flat,
linear, number-line graph. We can visualize our set-union partial order as a (much more interesting)
graph as well: (this is called a [Hasse diagram](https://en.wikipedia.org/wiki/Hasse_diagram))

||
| :---: |
| ![A graph showing the partial order of set-union with elements x, y, z. At the bottom is empty set, second row has singleton sets, third row has pairs, and top has a set with all three.](../img/set-union-ord.png) |
| A visualization of the set-union partial order over three elements, $x, y, z$. [By KSmrq](https://commons.wikimedia.org/wiki/File:Hasse_diagram_of_powerset_of_3.svg) |

The directed paths represent things getting _larger_.
In the diagram, $\{x\}$ is less (smaller) than $\{x, y, z\}$, represented as a path upwards from
the former to the later. In contrast, there is no path between $\{x,y\}$ and $\{y,z\}$ for example,
so they are incomparable.

The _merge_ function is also called the _least upper bound_ (LUB). This name comes from the partial
order interpretation. When merging two elements, the result is the smallest (least) item that is
still greater than both elements. Hence _least upper bound_.

---

## Reference

This section is a quick reference for people already comfortable with abstract algebra basics.

### Lattices

A join-semilattice with domain $S$ with $a, b, c \in S$ and join (or "merge") function
$\sqcup$ has the following properties:
$$
    a\sqcup(b\sqcup c) = (a\sqcup b)\sqcup c \quad\quad\quad\mathrm{\textit{(associative)}} \\
    \quad\quad\quad
    a\sqcup b = b\sqcup a \quad\quad\quad\quad\quad\mathrm{\textit{(commutative)}} \\
    \quad\quad
    a\sqcup a = a \quad\quad\quad\quad\quad\quad\mathrm{\textit{(idempotent)}} \\
$$
(Separately, meet-semilattices and join-semilattices are equivalent structures)

The join function creates a partial order $\sqsubseteq$:
$$
    a \sqsubseteq b \quad\equiv\quad a \sqcup b = b
    \quad\quad\quad\mathrm{\textit{(semilattice partial order)}}
$$
Read as "$a$ preceedes $b$", or "$b$ dominates $a$".

The smallest element in a lattice domain $S$, if it exists, is the _bottom_, $\bot$:
$$
    \forall a\in S,\quad \bot \sqsubseteq a
    \quad\quad\quad\mathrm{\textit{(bottom)}}
$$
The largest element in a lattice domain $S$, if it exists, is the _top_, $\top$:
$$
    \forall a\in S,\quad \top \sqsupseteq a
    \quad\quad\quad\quad\mathrm{\textit{(top)}}
$$

### The CALM Theorem and Monotonicity

The [CALM Theorem _(Consistency As Logical Monotonicity)_](https://cacm.acm.org/magazines/2020/9/246941-keeping-calm/fulltext)
tells us: "a program has a consistent, coordination-free distributed
implementation if and only if it is monotonic"

A function $f: S\rightarrow T$ is _monotonic_ if it preserves a partial ordering of its domain to a
(possibly different) partial ordering of its codomain.
$$
    a \sqsubseteq_S b \quad\Longrightarrow\quad f(a)\ \sqsubseteq_T\ f(b)
    \quad\quad\quad\mathrm{\textit{(monotonicity)}}
$$

### Lattice Morphism

A function $f: S\rightarrow T$ from lattice domain $S$ to lattice codomain $T$ is a _morphism_ if
it structurally preserves merges, i.e. merges distribute across the function. For all $a,b\in S$:
$$
    f(a \sqcup_S b) \quad=\quad f(a) \sqcup_T f(b)
    \quad\quad\quad\mathrm{\textit{(morphism)}}
$$
(Because both the domain and codomain are semilattice spaces, _semilattice homomorphism_ is the
most precise term for this.)

Lattice morphisms are a special kind of monotonic function which are _differentially computable_.
Because merge distributes over a morphism, we can evaluate the morphisms on a small "delta" of data
and merge that delta into the existing result rather than recompute the entire morphism on all data.

### Further Reading

* [Hydroflow Thesis (2021)](https://hydro.run/papers/hydroflow-thesis.pdf)
