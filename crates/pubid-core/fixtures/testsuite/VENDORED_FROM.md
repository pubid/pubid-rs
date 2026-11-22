# Vendored fixture provenance

The flavor directories beside this note (doi, isbn, iana, oasis, xsf,
un, ogc, ietf, w3c, ecma, gb, gost, plateau, evs, ccsds,
cen_cenelec, jis, etsi) hold the byte-copied pubid-testsuite corpora (see
`/pins.yaml`, `testsuite_ref`). Every slice was copied from
`pubid/pubid-testsuite@e6cf11b9821929f2c4deed431113bdfca67e81d6`
(`tests/<flavor>/`). The doi slice is byte-identical across the round
(`adecf10cef5c` -> `e6cf11b98219` touched only `tests/iso`), and the
wave-2 corpora are identical from `adecf10cef5c` forward, so the
bootstrap measurements carry over unchanged. (The api, amca, and other
later slices came from the same testsuite tree in the rounds that
ported them; nesc and ire are vendored-only, with no `tests/<flavor>/`
corpus at the pinned ref; the eu + arxiv slices at the end of this
note are the generated exception, and the ansi slice is the
regenerated exception, also at the end of this note.)

The slices make the conformance runs self-contained (`cargo test` needs
no network). CI additionally runs the conformance binary against a full
clone of the testsuite at the pinned ref, and fails the build when any
file drifts from that ref.

File sha256s at the pinned ref:

- ecma:
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `memento.yaml` sha256 a4fbb12f195a199daa7fa6a2860fd0c7b88e80500ee85e75636e5aa5eb1c3a48
  - `standard.yaml` sha256 ce741224d0e0a489edaf93428eaa2868cb22e8ca9ac0c37c06d6800fdf0e1c2c
  - `technical_report.yaml` sha256 23952bcbe3b972f70ac2a11fd04881388ad75d5f6ac606477e12226535f02827
- gb:
  - `_negative.yaml` sha256 183e20b3fbf98cdcf8fabe91887ebd7dba300e0a982cc14d79ccb24b36c95d29
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `all_parts.yaml` sha256 95ea69aebd8d1962b48bfb2b22ccec31fd484913cb37ed1976550501a8fe1e66
  - `standard.yaml` sha256 26bec7ab6cd6eae89a304fe774f2c489d7bdcff372b4050696f1a000f3caeb0e
- gost:
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `harmonized.yaml` sha256 b00453247e0cf93305796cd9808b742693541c6cbd5bb6077ab1bd3711317ffd
  - `identical_adoption.yaml` sha256 1e3b57761f0a591d5649c72194875cec71b8a0a5413b48fed27681ad1faa7011
  - `interstate_standard.yaml` sha256 a00d9e9ab2752c7c9ef29a3d0d9a743074ec6881229018bef345abca29194798
  - `national_standard.yaml` sha256 66c68be7d8b0fe9c152e02a3cd64859ccfd596541d0defc28041b00b07e449f4
- doi:
  - `_negative.yaml` sha256 0a14710dcc63dfc47200fdd7d7b10bfd2e039a8ec52e748cffc78820adeaa44c
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `resource.yaml` sha256 31d7941c4e1465fde00e1ab92469a9a222bce84a214bf35f59fb9ad766e97878
- isbn:
  - `_negative.yaml` sha256 9373315c983fd1b9b9dac2a39112c79bc98192086d7c0bb6f4e4d1c706f4fe89
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `book.yaml` sha256 6fad7222912f582f7a099382d969debf6e9f368946d27663a61be502422e5e84
- iana:
  - `_negative.yaml` sha256 d0f6e1bc8d7e6e674e360fe6e9132c015d1c3bf69605a7ae052f0888443db595
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `registry.yaml` sha256 2077673fe12fb728fc31ede8ff56117265cdcea5d35925aa6470a3fe07d56251
- oasis:
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `standard.yaml` sha256 703f0030c651c16f698c19771e8871903a71292518c2c516ea4d63c769729920
- xsf:
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `xep.yaml` sha256 98714413184a63862f42e1640302654f7af65be9df4a43519e60fb9d37b11ca8
- un:
  - `_negative.yaml` sha256 cdc4f70c96b63573f51c24431364cffa981b46b846926f7197aac2e2d7c99c8f
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `document.yaml` sha256 c0169b7969e3fc58b3bc76f4966efb97e3f784d8e0b8726cbb4222e85dfe8a6c
- ogc:
  - `_negative.yaml` sha256 93443b8089887f56af615bf5f44f7f43805c2b0da5433b2e8d18380e22d67815
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `document.yaml` sha256 ed599cc8b90aa9dff82787271aea54b2516c90331cc6f8d125a21a96f4ca7cd3
- ietf:
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `bcp.yaml` sha256 5550f54365fa3abe45c266d956fc5c0bc239fb1e652ca3abc845ea1d8d5bf764
  - `fyi.yaml` sha256 8ec558a40b6462fb419fb5049e5d8217452058611312f6ca14a88f566dc6051a
  - `internet_draft.yaml` sha256 096c4e2216faf1f9983947a7602e74abc9e7e0c8c3eb648a1b521da621976138
  - `rfc.yaml` sha256 052b2b9cd92ed30cebce97bb855092ae47c74c8240c712456434d694c3130260
  - `std.yaml` sha256 3a320f9cb2fdacb6bdc169971c2bb4300db7c0a986592886e5f2ceda0125ba01
- w3c:
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `candidate_recommendation.yaml` sha256 170cfca8456f117b438f608c9d6e3f98a3004d51d5b55fff1d7935a41e8a68d5
  - `candidate_recommendation_draft.yaml` sha256 fefcecf7b144eb7c80ccfd4666b94148ff526c95117824f8eb95858d2822e9fe
  - `draft_note.yaml` sha256 d94fccc6255984c7f6df6c69e2740123f6a502d111c02186dbf941c513917013
  - `note.yaml` sha256 6fee88c73048cd80cc5ca71022994daad17e1e8d0b81cd30969df12e6ccb91a0
  - `obsolete_recommendation.yaml` sha256 6a0da68507f6dc90c53a5389ac58e9b904610d9dfffa9c4370e1f48d36550bb3
  - `proposed_edited_recommendation.yaml` sha256 7319c3210fcd813ed4f6fe50a11c4dde22806d463fe7f2e73ce3be7c45868272
  - `proposed_recommendation.yaml` sha256 a830895831e1858e2fbf20df4cd1f5322aafbeb5e3e7a577014a9c45c4f1860e
  - `recommendation.yaml` sha256 3aae6a5afed66831bb1ce678652d06947f0a44b64f3adfdd51127064f6860c1a
  - `standard.yaml` sha256 b54aa01bd8c2e1291bcae919aa2f1b6fe22d4088bbd135d71cae0badb153938d
  - `superseded_recommendation.yaml` sha256 44408531c260655b5464e30eaeea70de4e12ec5fc6f943511b1bd0def2643378
  - `working_draft.yaml` sha256 3ca41c77bf58b69916c62c3e54bc6bc2175e5081faf5fe376f96ec508f531dca

- plateau:
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `handbook.yaml` sha256 f66c0dc26a14b56ef0bdff6313bf9d8121ad69527b8986935aef5d5d4ddc4d75
  - `technical_report.yaml` sha256 cb4bab1b5862b65a2754742a24b7e7f9abef233953d3779db30cef98b2249f6e
- evs:
  - `_status.yaml` sha256 5ebef97c51fd67ec6371962b5790985ecb651511ee327b410cd1b66d1b4a3a37
  - `estonian_standard.yaml` sha256 a203f4300ff9d22f4678b8ed1c9b8018f0d74ac268d67d086c4dfee02fa2be70
  - `international_adoption.yaml` sha256 9c8aae8e0b1065faf7e6f02b33a8881998f143536db642a1d7251221319d724b
  - `national_adoption.yaml` sha256 bc05b88adcdf040800c6818ab9af7e916b90e8db0faaab62c2581b7d23404a8d
- ccsds:
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `base.yaml` sha256 e2aa9a974c844b4f5e6fe6af3c44a80471b2098d684251f7f4b1a65a800bb722
  - `corrigendum.yaml` sha256 ee67d26b75500074cc371ac1388590e72482df8bdf889870824d4901eb5f8b92
- cen_cenelec:
  - `_negative.yaml` sha256 0ab1ea4f5dae51f9c903894493c887c43ee5903e47ea4d0bce0cd05bbb6b1657
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `adopted_european_norm.yaml` sha256 54b2b21ac2984b2f9bf2927ef8abbc2157c5f47d5e75b93ebdc3f6f5c6e3c70d
  - `amendment.yaml` sha256 bae0306da1d6d9293ecaf9a1535ceb1c497d64177a94ff699a23cec76c2ee546
  - `cen_report.yaml` sha256 4293cd8cf3349fcead5b33298ae0968a83e330cb9f02915b91d854d69f41d2ab
  - `cen_workshop_agreement.yaml` sha256 d768e1a466960d920e70d33b5cb58851e5377af61c1931f89a7ba38e037ccb2c
  - `consolidated_identifier.yaml` sha256 d8139971114fc429837e9fffb5a9b00c6bdd9db6a6ac74c0f61fd1f2d05ddff1
  - `corrigendum.yaml` sha256 beb79e791d07e06957305c6293c4dbc0e39a3fef5c9c9250d65b436883802c62
  - `european_norm.yaml` sha256 55e45e0453c6b8e3761984afefe92c94ebe5026f6c7e2b5e1f21a813c3f79221
  - `european_prestandard.yaml` sha256 fd466ebd7585590ba72be40c4f399c515572c605e6929e8d3e3bae475e878d9d
  - `european_specification.yaml` sha256 a16bf003ea29580c7fdcfaa05e98fe63dc4dcaa56df93041095145f4a8e37be1
  - `fragment.yaml` sha256 d4df6b2867072f07f5eafdc1373890be507dab1d411dd74f22c474ee265bfecc
  - `guide.yaml` sha256 c33884dc028dc4f63098b948a7e6a3e32704d1ea00c1276383e20de2956ae01f
  - `harmonization_document.yaml` sha256 68bf654b764f3fccee3d2c4ce1a5b114f62008d7f82109fcaa73c2e3854941a5
  - `technical_report.yaml` sha256 d7f73dea933acf66c1c7b9db46f04c8b6b7746de14e918e004716da2f9ac2b5e
  - `technical_specification.yaml` sha256 7e14414db57836767e9a8ee657d3d89cbd885e41a3c4178a8d5b9b9b9445a094
- jis:
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `amendment.yaml` sha256 298dd02bf2fced7a25b498198e22833d10348bfde126ba20c6f385d9a23eb7d9
  - `japanese_industrial_standard.yaml` sha256 fe34dc03ed5d63e820710a8b54d86c99d80add1bc06466a95fe035000cc22354
  - `welding_engineering_standard.yaml` sha256 fb30d3e9201e04e12eec60727188b003cdfd6a8d00d8cfbb33527afc8473f439
  - `welding_engineering_technical_report.yaml` sha256 0ec0ae5400a66658821905e5402d15eb8ed1b41519dcdbea1d939b4889ee6699
  - `welding_engineering_technical_specification.yaml` sha256 b052559bca87779213b98e1ec46396cff8774f7d92b7e2386862226a083a927c
- etsi:
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `amendment.yaml` sha256 f6345d930f69f46e79c884e2e4db5a2ac82d4727192e7e398f089bb62d4b3c3c
  - `corrigendum.yaml` sha256 e4cb1d205183237c23e1dd557ed2429cbbb5d6071304dc9b49065d4e82f2c486
  - `etsi_standard.yaml` sha256 9a9a738934a96b66bdc699a659b20a09585ba246abc7d70127da3296225213f9

The eu + arxiv slices are the one exception: the testsuite at the
pinned ref carries no `tests/eu/` or `tests/arxiv/` yet (the flavor
corpora live in the gem's `spec/fixtures/{eu,arxiv}/identifiers/`),
so these slices were generated in-process from the reference itself —
the pubid gem at `fe08a252` (2.0.44, the `Pubid::Eu` +
`Pubid::Arxiv` flavors), running its own `spec/fixtures` rows:

- every row's canonical hash, human, and URN came from the reference
  (parsed, then serialized by the flavor class);
- every alias spelling (`!src!dst` fixture rows) was verified to
  render to its row's human in the reference before it was written;
- every `_negative.yaml` input was verified to raise
  `Pubid::Errors::ParseError` in the reference;
- the row ids are slice-local (`eu.regulation.0001`...), assigned in
  the fixtures' own order — when the testsuite exports the two
  flavors, replace these files with the byte copy and fold them back
  into `testsuite_ref` (the slice-cmp drift gate skips them today:
  nothing exists at the pinned ref to drift against).

Generated-slice file sha256s at pubid fe08a252:

- eu:
  - `_negative.yaml` sha256 43d655dff2b8ebc52887745ba6ded8f141998e9e31da20fd45fde9edd71b12da
  - `_status.yaml` sha256 d0de93e4b73ccba08ed2eec9a3b0f8b2b4a9afa11749d2e7c7869298795fff70
  - `decision.yaml` sha256 7ddcd25d0850ab021298a00a77f0790284f1937d3d752cb9f189747330568730
  - `directive.yaml` sha256 4f550cc6560cce93683422cd68414fe5a47c3faad05b36bfb680fadc42fa2e61
  - `regulation.yaml` sha256 5af266cb4b4b1574e6833a8327e9fbed6b59a37e161cb6b7636ac14eb181e9bb
- arxiv:
  - `_negative.yaml` sha256 0e8e83a6d1f4d45a053797ba3d3b36b7623b151f57d5c594cbae8e7a7fa8dcf6
  - `_status.yaml` sha256 d0de93e4b73ccba08ed2eec9a3b0f8b2b4a9afa11749d2e7c7869298795fff70
  - `eprint.yaml` sha256 56b57ea7ce489b5367d60c80a68f9606cf12493e400930087a6384aa0c4bc896

Refresh by re-copying the files from the new testsuite ref and updating
`testsuite_ref` in `/pins.yaml` together with this note.

The ansi slice is the regenerated exception. The reference moved under
it (pubid a6c7f8b5, 2.0.47's ANSI trailing-year + open co-publisher
round, pubid#760 / grammar#139) while the testsuite still carries the
pre-#760 `tests/ansi/` at the pinned ref: 170 of the 175 rows record
the trailing `-YYYY` as `part` and no `year`, which the reference now
parses as the year + `date_separator`. The slice was therefore
regenerated in-process from the reference itself — the pubid gem at
`a6c7f8b5` (checked out at 2.0.46 + the ANSI commit), running the
gem's own `spec/fixtures/ansi/identifiers/pass/standard.txt` rows:

- all 192 ids (the 175 testsuite rows re-recorded at the moved
  expectations, in their row order, plus the 17 ids the gem's fixture
  round added): every row's canonical hash, human, and URN came from
  the reference (parsed, re-parsed from the render, then serialized by
  the flavor class — the exporter's one-parse rule);
- ids stay position-stable (`ansi.standard.0001`-`0175` keep their
  inputs), so the 170 moved rows keep the ids the gem's
  `conformance/pending.yaml` marks;
- the row shape is the exporter's (`identifier` / `representations` /
  `style: v3`, Ruby psych `YAML.dump` byte style).

The slice-cmp drift gate skips ansi until the testsuite exports the
ANSI year round; then replace these files with the byte copy, fold
them back into `testsuite_ref`, clear the pending marks, and drop this
section.

Generated-slice file sha256s at pubid a6c7f8b5 (parsanol 1.5.12
runtime over the grammar#139 artifact):

- ansi:
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `standard.yaml` sha256 6c97c938b3584238609067ee12ec57f6b49bcb45b1ba4fc170f06fce0b1da1ac

The wmo slice is a byte copy, but from its own flavor-only testsuite
ref: `pubid/pubid-testsuite@0c9ddd7fab0dc1386f77ca121150998a3ca28320`
(`tests/wmo/`, the #763 round freeze at pubid 0f7ba38b1658 — see
`/pins.yaml`, `testsuite_ref_wmo`). The span from `testsuite_ref`
(e6cf11b) to that ref re-exports ansi and adds the eu/arxiv corpora,
which this repository's regenerated slices do not fold back yet, so
the wmo slice is checked against its own ref (the dedicated CI drift
gate) and the wmo conformance run gates against the
`pubid-testsuite-wmo` checkout. Everything here is the testsuite's
export, unmodified (clean status, no debt).

- wmo:
  - `_negative.yaml` sha256 1a53d8985b965b3eb01c83ac693a0d8a70d1ad8d87a82e3d1d8cfef83b793de1
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `annex.yaml` sha256 b6a7fa4e7953e636dd9ff788831ca460bf911e8ab8ab465b42b36ec65d3dde18
  - `publication.yaml` sha256 9582c2c8a2311398114dcb8ee1e123f8a83e5edfb5801f3144cdd4ae96d82e1e
  - `standard.yaml` sha256 e7b7d313dd7d4d22991a40481901e4bd892d4b74f7f0f763d276ab5fa763faac
