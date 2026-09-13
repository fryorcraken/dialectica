//! Refused noun-place pairs: the compositions that spell a real figure's
//! canonical name.
//!
//! **Generated — do not edit by hand.** Built from `tmp/attributions.txt`,
//! which lists each named historical Greek in the noun list beside the places
//! that figure is canonically cited with.
//!
//! # Why this family is mandatory rather than discretionary
//!
//! The *X of Y* shape can produce exactly how a historical figure is
//! conventionally cited — *straton of lampsakos* is how Straton of Lampsacus is
//! actually referred to — so a user drawing that pair has every post they make
//! signed with a real person's full canonical identifier. That is a structural
//! property of the shape, not the separate exclusion of a handful of figures.
//!
//! # This is not a screen on meaning, and not an exclusion
//!
//! **Both halves of a refused pair stay in their lists and draw freely
//! elsewhere.** No entry is kept out of any list for what it says, what it
//! connotes or whom it names. What is refused is a *composition* that states a
//! falsehood about who is posting — an attribution rather than a tone — and it
//! is refused because the pair is enumerable by lookup rather than judged word
//! by word. Removing either word instead would cost two entries per figure,
//! buy nothing, and be exactly the exclusion the contract forbids.
//!
//! # Completeness is a curation claim, not a testable one
//!
//! A test can assert this list is sorted, deduplicated, in range, and that the
//! pairs on it are refused. It **cannot** assert that a pair missing from it
//! should have been on it — that is a claim about the world. A missed pair
//! renders one identity under a real person's name; it is a curation gap rather
//! than a code defect, and it cannot be repaired after release without a scheme
//! version bump.
//!
//! Sorted, so `binary_search` is correct and its precondition is a property a
//! test can assert — which a `HashSet` would hide.

pub const TRUE_ATTRIBUTION_PAIRS: &[(u16, u16)] = &[
    (11, 436),   // agatharchides of knidos
    (26, 437),   // ainesidemos of knossos
    (29, 895),   // aischines of sphettos
    (38, 109),   // akousilaos of argos
    (43, 628),   // alkaios of mytilene
    (49, 847),   // alkman of sardis
    (49, 893),   // alkman of sparta
    (63, 943),   // anakreon of teos
    (67, 433),   // anaxagoras of klazomenai
    (68, 603),   // anaximandros of miletos
    (69, 505),   // anaximenes of lampsakos
    (69, 603),   // anaximenes of miletos
    (76, 491),   // andronikos of kyrrhos
    (76, 836),   // andronikos of rhodos
    (81, 997),   // anthemios of tralleis
    (86, 925),   // antipatros of tarsos
    (88, 827),   // antiphon of rhamnous
    (95, 444),   // apelles of kolophon
    (95, 460),   // apelles of kos
    (99, 131),   // apollodoros of athenai
    (101, 740),  // apollonios of perge
    (101, 836),  // apollonios of rhodos
    (105, 866),  // aratos of sikyon
    (105, 888),  // aratos of soloi
    (107, 715),  // archilochos of paros
    (108, 915),  // archimedes of syrakousai
    (117, 844),  // aristarchos of samos
    (117, 845),  // aristarchos of samothrake
    (119, 490),  // aristippos of kyrene
    (120, 179),  // ariston of chios
    (121, 170),  // aristophanes of byzantion
    (122, 896),  // aristoteles of stageira
    (124, 780),  // arkesilaos of pitane
    (125, 643),  // arrianos of nikomedeia
    (126, 242),  // artemidoros of ephesos
    (149, 780),  // autolykos of pitane
    (152, 408),  // bakchylides of keos
    (159, 160),  // bion of borysthenes
    (159, 886),  // bion of smyrna
    (172, 540),  // chares of lindos
    (176, 893),  // cheilon of sparta
    (180, 437),  // chersiphron of knossos
    (189, 888),  // chrysippos of soloi
    (201, 451),  // deinarchos of korinthos
    (202, 836),  // deinokrates of rhodos
    (206, 2),    // demokritos of abdera
    (218, 48),   // didymos of alexandria
    (221, 17),   // diodoros of agyrion
    (221, 344),  // diodoros of iasos
    (222, 392),  // diokles of karystos
    (224, 297),  // dionysios of halikarnassos
    (226, 48),   // diophantos of alexandria
    (229, 868),  // diphilos of sinope
    (253, 38),   // empedokles of akragas
    (267, 481),  // ephoros of kyme
    (268, 460),  // epicharmos of kos
    (268, 915),  // epicharmos of syrakousai
    (270, 844),  // epikouros of samos
    (272, 437),  // epimenides of knossos
    (279, 408),  // erasistratos of keos
    (281, 490),  // eratosthenes of kyrene
    (297, 836),  // eudemos of rhodos
    (298, 436),  // eudoxos of knidos
    (299, 48),   // eukleides of alexandria
    (299, 578),  // eukleides of megara
    (303, 451),  // euphranor of korinthos
    (320, 738),  // galenos of pergamon
    (322, 836),  // geminos of rhodos
    (332, 522),  // gorgias of leontinoi
    (354, 2),    // hekataios of abdera
    (354, 603),  // hekataios of miletos
    (359, 527),  // hellanikos of lesbos
    (359, 628),  // hellanikos of mytilene
    (363, 242),  // herakleitos of ephesos
    (365, 628),  // hermarchos of mytilene
    (368, 925),  // hermogenes of tarsos
    (369, 48),   // herodianos of alexandria
    (370, 297),  // herodotos of halikarnassos
    (371, 48),   // heron of alexandria
    (373, 174),  // herophilos of chalkedon
    (375, 121),  // hesiodos of askra
    (378, 48),   // hesychios of alexandria
    (381, 48),   // hierokles of alexandria
    (383, 642),  // hipparchos of nikaia
    (384, 235),  // hippias of elis
    (386, 179),  // hippokrates of chios
    (386, 460),  // hippokrates of kos
    (388, 242),  // hipponax of ephesos
    (405, 48),   // hypatia of alexandria
    (414, 175),  // iamblichos of chalkis
    (420, 829),  // ibykos of rhegion
    (432, 603),  // isidoros of miletos
    (437, 603),  // kadmos of miletos
    (444, 490),  // kallimachos of kyrene
    (445, 242),  // kallinos of ephesos
    (456, 490),  // karneades of kyrene
    (489, 124),  // kleanthes of assos
    (492, 866),  // kleisthenes of sikyon
    (494, 540),  // kleoboulos of lindos
    (507, 505),  // kolotes of lampsakos
    (517, 888),  // krantor of soloi
    (519, 131),  // krates of athenai
    (519, 566),  // krates of mallos
    (519, 954),  // krates of thebai
    (529, 755),  // kritolaos of phaselis
    (532, 436),  // ktesias of knidos
    (533, 48),   // ktesibios of alexandria
    (549, 893),  // leonidas of sparta
    (549, 923),  // leonidas of taras
    (552, 2),    // leukippos of abdera
    (552, 229),  // leukippos of elea
    (552, 603),  // leukippos of miletos
    (554, 235),  // libon of elis
    (560, 175),  // lykophron of chalkis
    (568, 866),  // lysippos of sikyon
    (578, 636),  // marinos of neapolis
    (592, 844),  // melissos of samos
    (598, 48),   // menelaos of alexandria
    (607, 179),  // metrodoros of chios
    (607, 505),  // metrodoros of lampsakos
    (607, 876),  // metrodoros of skepsis
    (609, 444),  // mimnermos of kolophon
    (620, 915),  // moschos of syrakousai
    (624, 232),  // myron of eleutherai
    (643, 444),  // nikandros of kolophon
    (663, 48),   // olympiodoros of alexandria
    (676, 738),  // oribasios of pergamon
    (687, 586),  // paionios of mende
    (693, 836),  // panaitios of rhodos
    (698, 48),   // pappos of alexandria
    (703, 229),  // parmenides of elea
    (705, 242),  // parrhasios of ephesos
    (706, 642),  // parthenios of nikaia
    (711, 866),  // pausias of sikyon
    (719, 451),  // periandros of korinthos
    (729, 235),  // phaidon of elis
    (736, 131),  // pherekydes of athenai
    (736, 916),  // pherekydes of syros
    (739, 888),  // philemon of soloi
    (739, 915),  // philemon of syrakousai
    (744, 48),   // philon of alexandria
    (744, 170),  // philon of byzantion
    (744, 510),  // philon of larisa
    (745, 577),  // philopoimen of megalopolis
    (746, 48),   // philoponos of alexandria
    (749, 493),  // philoxenos of kythera
    (751, 997),  // phlegon of tralleis
    (768, 954),  // pindaros of thebai
    (770, 628),  // pittakos of mytilene
    (780, 131),  // plutarchos of athenai
    (780, 171),  // plutarchos of chaironeia
    (787, 131),  // polemon of athenai
    (791, 505),  // polyainos of lampsakos
    (792, 577),  // polybios of megalopolis
    (794, 952),  // polygnotos of thasos
    (796, 109),  // polykleitos of argos
    (806, 836),  // poseidonios of rhodos
    (808, 460),  // praxagoras of kos
    (817, 408),  // prodikos of keos
    (818, 131),  // proklos of athenai
    (826, 2),    // protagoras of abdera
    (829, 402),  // protogenes of kaunos
    (833, 48),   // ptolemaios of alexandria
    (840, 235),  // pyrrhon of elis
    (841, 844),  // pythagoras of samos
    (843, 800),  // pytheos of priene
    (852, 844),  // rhoikos of samos
    (857, 251),  // sappho of eresos
    (857, 527),  // sappho of lesbos
    (857, 628),  // sappho of mytilene
    (866, 67),   // semonides of amorgos
    (872, 408),  // simonides of keos
    (873, 422),  // simplikios of kilikia
    (877, 715),  // skopas of paros
    (886, 242),  // soranos of ephesos
    (887, 436),  // sostratos of knidos
    (902, 322),  // stesichoros of himera
    (910, 505),  // straton of lampsakos
    (922, 490),  // synesios of kyrene
    (957, 603),  // thales of miletos
    (964, 490),  // theodoros of kyrene
    (964, 844),  // theodoros of samos
    (965, 578),  // theognis of megara
    (966, 915),  // theokritos of syrakousai
    (967, 251),  // theophrastos of eresos
    (968, 179),  // theopompos of chios
    (981, 174),  // thrasymachos of chalkedon
    (985, 928),  // timaios of tauromenion
    (986, 763),  // timon of phlious
    (987, 603),  // timotheos of miletos
    (1003, 48),  // tryphon of alexandria
    (1010, 893), // tyrtaios of sparta
    (1011, 174), // xenokrates of chalkedon
    (1012, 444), // xenophanes of kolophon
    (1013, 242), // xenophon of ephesos
    (1013, 460), // xenophon of kos
    (1015, 229), // zenon of elea
    (1015, 431), // zenon of kition
    (1015, 925), // zenon of tarsos
];
