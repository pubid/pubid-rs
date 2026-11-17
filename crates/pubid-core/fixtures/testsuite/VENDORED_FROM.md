# Vendored fixture provenance

The flavor directories beside this note (doi, isbn, iana, oasis, xsf,
un, ogc, ietf, w3c, ansi, ecma, gb, gost) hold the byte-copied pubid-testsuite corpora (see
`/pins.yaml`, `testsuite_ref`). Every slice was copied from
`pubid/pubid-testsuite@e6cf11b9821929f2c4deed431113bdfca67e81d6`
(`tests/<flavor>/`). The doi slice is byte-identical across the round
(`adecf10cef5c` -> `e6cf11b98219` touched only `tests/iso`), and the
wave-2 corpora are identical from `adecf10cef5c` forward, so the
bootstrap measurements carry over unchanged.

The slices make the conformance runs self-contained (`cargo test` needs
no network). CI additionally runs the conformance binary against a full
clone of the testsuite at the pinned ref, and fails the build when any
file drifts from that ref.

File sha256s at the pinned ref:

- ansi:
  - `_status.yaml` sha256 da8b5c732041f6936d83e8f046b33824cea9c87e149da4d107ae8efd143a050f
  - `standard.yaml` sha256 0f0ac3afbc09e16dafa39aae833fad25e9ecff76c1671474eba277f25aee4c38
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

Refresh by re-copying the files from the new testsuite ref and updating
`testsuite_ref` in `/pins.yaml` together with this note.
