# Lady Deirdre Examples, Benchmarks, Integration Test.

This subproject of the Lady Deirdre technology contains a simple example of the
[Json incremental parser](https://github.com/Eliah-Lakhin/lady-deirdre/tree/master/work/crates/examples/src/json),
[performance Benchmarks](#benchmarks), and
[Integration Tests](https://github.com/Eliah-Lakhin/lady-deirdre/tree/master/work/crates/examples/tests).

## Benchmarks.

### Setup.

The code of the Benchmark tests is under the
[benches](https://github.com/Eliah-Lakhin/lady-deirdre/tree/master/work/crates/examples/benches)
directory.

These tests generate a set of random
[JSON](https://en.wikipedia.org/wiki/JSON) files of different sizes and nesting
complexity, and series of random edits(insertions, deletions and replacements)
of also different sizes and nesting complexity for each JSON files.

The JSON files and each edit results are always valid JSONs, and the validity
is verified automatically beforehand.

The Benchmarks test computational performance on each series of edits comparing
three well-known Rust frameworks of different functional capabilities, and the
two different instances of Lady Deirdre:
 - [Nom](https://crates.io/crates/nom). A parsers combinator library. This
   combinator library is widely recognized as one of the best in performance
   for non-incremental parsing, but Nom does not have any incremental reparsing
   capabilities.
 - [Tree-Sitter](https://crates.io/crates/tree-sitter). An incremental parsers
   generator tool. This library is one of the most widely recognizable solution
   for incremental parsing.
 - [Ropey](https://crates.io/crates/ropey). A mutable text storage library.
   This library does not provide any syntax or lexis parsing capabilities, but
   Ropey has mutable text storage functionality similar to some Lady Deirdre
   functions.
 - "Self Syntax" is an instance of the Json syntax and lexis incremental parser
   that uses Lady Deirdre under the hood.
 - "Self Lexis" is an instance of the Json lexis only incremental parser
   that uses Lady Deirdre under the hood.

There are three series of tests on three independent JSON files of different
sizes:
 - ~10Mb file (10791089 bytes, 178508 lines).
 - ~4Mb file (4346095 bytes, 72072 lines).
 - ~82 Kb. (84638 bytes, 1957 lines).

For each file the benchmarks test initial loading time, independent edits time,
and the series of edits(1100 total edits) applied sequentially.

The series of edits is the most interesting performance indicator, because it
shows actual live editing of the text that in some way mimics end-user
sequential edit actions.

I used my mobile Intel NUC machine to perform benchmark tests:
Intel Core i7-8809G CPU @ 3.10GHz × 8, 16Mb RAM.

### Results.

You can find complete Criterion report
[here](https://6314c0d3ffd9447cb096168e--cheerful-malasada-35b65a.netlify.app/report/).

1. **Incremental Reparsing.**

   Lady Deirdre shows almost the same performance on the small file(82Kb)
   sequential edits as Tree-Sitter does: 12.1ms vs 11.25ms.

   But Lady Deirdre demonstrates significantly better results than Tree-Sitter
   on medium(4Mb) and large(10Mb) files: 18ms vs 58ms and 39.1ms vs 124.5ms
   accordingly.

2. **Non-Incremental Parsing.**

   Nom works significantly better than Tree-Sitter and Lady Deirdre for initial
   parsing. For the small file(82Kb) Nom has parsed the file in 0.87ms,
   Lady Deirdre in 2.48ms, and Tree-Sitter in 5.91ms.

   For the larger file(10Mb) Nom's results are comparable too:
   87.25ms(Nom) vs 304ms(Lady Deirdre) vs 624ms(Tree-Sitter).

   Even though non-incremental parser combinator Nom shows significantly
   better results that the incremental parsers, Lady Deirdre works up to 2 times
   faster in these tests than Tree-Sitter does.

   For non-incremental series of complete reparsing of the small JSON file Nom
   demonstrates expected performance degradation comparing to Lady Deirdre and
   Tree-Sitter both: ~2155ms for 1100 edits complete reparsing.

3. **Text Mutations.**

   Ropey demonstrates significantly better results on all text edit tests
   than Tree-Sitter and Lady Deirdre both (these results not applicable to Nom).
   To compare, on large JSON file(10Mb) a series of edits took
   1.26ms(Ropey) vs 11.1ms(Lady Deirdre JSON lexis only parser).

   For the fair comparison I would have to opt-out Lady Deirder's lexis parser
   in these tests, but this is currently not possible.

### Conclusion.

Lady Deirdre demonstrates better performance than Tree-Sitter on initial data
loading on all tests, comparable performance of incremental reparsing on small
files, and better performance on incremental reparsing on medium to large files.

These results allow me to conclude that in certain applications Lady Deirdre
could be a competitive replacement to Tree-Sitter, a widely used
production-ready incremental parsing solution. However, it is worth to mention
that both solutions have different and sometimes incomparable functional
capabilities, and the different goals. Moreover, the tests performed in these
Benchmarks were applied on merely artificial snippets and relatively simple Json
syntax.

For non-incremental parsing Nom and the solutions of the same class are
beneficial in performance for the application developers, however both
Tree-Sitter and Lady Deirdre are still applicable solutions for this type
of parsing too.
