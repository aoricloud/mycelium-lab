//! Croissance du mycélium : apex, marche persistante et ramification.
//!
//! # Le modèle
//!
//! Des apex vivent sur le plan, chacun avec une position et une direction. À chaque pas de temps, chaque apex, dans
//! l'ordre de son indice :
//!
//! 1. tourne : sa direction change d'un petit angle entier tiré au hasard autour de la précédente (marche
//!    persistante) ;
//! 2. avance d'une longueur [`Parametres::pas`] dans cette direction et laisse un [`Segment`] d'hyphe ;
//! 3. se ramifie avec la probabilité [`Parametres::probabilite_ramification`] : un nouvel apex naît à la nouvelle
//!    position, sa direction décalée de [`Parametres::angle_ramification`] d'un côté ou de l'autre (le côté tiré au
//!    hasard). L'apex d'origine continue sa route ; le nouveau apex n'avance qu'à partir du pas suivant.
//!
//! # Les directions
//!
//! Une direction est un indice entier dans `0..NB_DIRECTIONS` (un degré par indice, en tournant dans le sens
//! trigonométrique depuis l'axe des x). Les cosinus et sinus viennent des tables [`COS`] et [`SIN`] écrites dans ce
//! fichier : aucun appel à `sin` ni à `cos`, donc le même résultat sur toutes les machines.
//!
//! # Le hasard
//!
//! Tout le hasard vient d'un [`Alea`] construit depuis la graine de la colonie. Le nombre et l'ordre des tirages
//! sont fixés par l'implémentation, mais ne dépendent que de la graine, des paramètres et du nombre de pas.

use crate::alea::Alea;

/// Nombre de directions de la table : une direction est un indice dans `0..NB_DIRECTIONS`, soit un degré par indice.
pub const NB_DIRECTIONS: usize = 360;

/// Taille en octets de l'écriture canonique d'un [`Segment`].
pub const OCTETS_PAR_SEGMENT: usize = 40;

/// Cosinus de chaque direction : `COS[d]` vaut le cosinus de `d` degrés, arrondi au `f64` le plus proche. Les
/// multiples de 90 degrés sont exacts (`COS[0] == 1.0`, `COS[90] == 0.0`).
pub const COS: [f64; NB_DIRECTIONS] = [
    1.0,
    0.9998476951563913,
    0.9993908270190958,
    0.9986295347545738,
    0.9975640502598242,
    0.9961946980917455,
    0.9945218953682733,
    0.992546151641322,
    0.9902680687415704,
    0.9876883405951378,
    0.984807753012208,
    0.981627183447664,
    0.9781476007338057,
    0.9743700647852352,
    0.9702957262759965,
    0.9659258262890683,
    0.9612616959383189,
    0.9563047559630354,
    0.9510565162951535,
    0.9455185755993168,
    0.9396926207859084,
    0.9335804264972017,
    0.9271838545667874,
    0.9205048534524404,
    0.9135454576426009,
    0.9063077870366499,
    0.898794046299167,
    0.8910065241883679,
    0.882947592858927,
    0.8746197071393957,
    0.8660254037844387,
    0.8571673007021123,
    0.848048096156426,
    0.838670567945424,
    0.8290375725550416,
    0.8191520442889918,
    0.8090169943749475,
    0.7986355100472928,
    0.7880107536067219,
    0.7771459614569709,
    0.766044443118978,
    0.754709580222772,
    0.7431448254773942,
    0.7313537016191705,
    0.7193398003386512,
    std::f64::consts::FRAC_1_SQRT_2,
    0.6946583704589973,
    0.6819983600624985,
    0.6691306063588582,
    0.6560590289905073,
    0.6427876096865394,
    0.6293203910498375,
    0.6156614753256583,
    0.6018150231520484,
    0.5877852522924731,
    0.5735764363510462,
    0.5591929034707468,
    0.5446390350150271,
    0.5299192642332049,
    0.5150380749100542,
    0.5000000000000001,
    0.4848096202463371,
    0.46947156278589086,
    0.4539904997395468,
    0.43837114678907746,
    0.42261826174069944,
    0.4067366430758002,
    0.3907311284892737,
    0.37460659341591196,
    0.3583679495453004,
    0.3420201433256688,
    0.32556815445715676,
    0.30901699437494745,
    0.29237170472273677,
    0.27563735581699916,
    0.25881904510252074,
    0.24192189559966767,
    0.22495105434386492,
    0.20791169081775945,
    0.19080899537654492,
    0.17364817766693041,
    0.15643446504023092,
    0.13917310096006547,
    0.12186934340514749,
    0.10452846326765346,
    0.08715574274765814,
    0.06975647374412523,
    0.052335956242943966,
    0.03489949670250108,
    0.0174524064372836,
    0.0,
    -0.017452406437283477,
    -0.034899496702500955,
    -0.05233595624294384,
    -0.06975647374412533,
    -0.08715574274765824,
    -0.10452846326765355,
    -0.12186934340514737,
    -0.13917310096006535,
    -0.1564344650402308,
    -0.1736481776669303,
    -0.1908089953765448,
    -0.20791169081775934,
    -0.22495105434386503,
    -0.24192189559966779,
    -0.25881904510252085,
    -0.27563735581699905,
    -0.29237170472273666,
    -0.30901699437494734,
    -0.32556815445715664,
    -0.3420201433256687,
    -0.35836794954530027,
    -0.37460659341591207,
    -0.39073112848927377,
    -0.40673664307580026,
    -0.42261826174069933,
    -0.4383711467890775,
    -0.4539904997395467,
    -0.4694715627858909,
    -0.484809620246337,
    -0.4999999999999998,
    -0.5150380749100543,
    -0.5299192642332048,
    -0.5446390350150271,
    -0.5591929034707467,
    -0.5735764363510462,
    -0.587785252292473,
    -0.6018150231520484,
    -0.6156614753256583,
    -0.6293203910498373,
    -0.6427876096865394,
    -0.6560590289905072,
    -0.6691306063588582,
    -0.6819983600624984,
    -0.6946583704589974,
    -0.7071067811865475,
    -0.7193398003386512,
    -0.7313537016191705,
    -0.743144825477394,
    -0.754709580222772,
    -0.7660444431189779,
    -0.7771459614569709,
    -0.7880107536067219,
    -0.7986355100472929,
    -0.8090169943749473,
    -0.8191520442889919,
    -0.8290375725550416,
    -0.8386705679454239,
    -0.848048096156426,
    -0.8571673007021122,
    -0.8660254037844387,
    -0.8746197071393957,
    -0.882947592858927,
    -0.8910065241883678,
    -0.898794046299167,
    -0.9063077870366499,
    -0.9135454576426008,
    -0.9205048534524404,
    -0.9271838545667873,
    -0.9335804264972017,
    -0.9396926207859083,
    -0.9455185755993168,
    -0.9510565162951535,
    -0.9563047559630355,
    -0.9612616959383189,
    -0.9659258262890682,
    -0.9702957262759965,
    -0.9743700647852351,
    -0.9781476007338057,
    -0.981627183447664,
    -0.984807753012208,
    -0.9876883405951377,
    -0.9902680687415704,
    -0.992546151641322,
    -0.9945218953682733,
    -0.9961946980917455,
    -0.9975640502598242,
    -0.9986295347545738,
    -0.9993908270190958,
    -0.9998476951563913,
    -1.0,
    -0.9998476951563913,
    -0.9993908270190958,
    -0.9986295347545738,
    -0.9975640502598242,
    -0.9961946980917455,
    -0.9945218953682733,
    -0.9925461516413221,
    -0.9902680687415703,
    -0.9876883405951378,
    -0.984807753012208,
    -0.981627183447664,
    -0.9781476007338056,
    -0.9743700647852352,
    -0.9702957262759965,
    -0.9659258262890683,
    -0.9612616959383189,
    -0.9563047559630354,
    -0.9510565162951536,
    -0.9455185755993167,
    -0.9396926207859084,
    -0.9335804264972017,
    -0.9271838545667874,
    -0.9205048534524404,
    -0.9135454576426009,
    -0.90630778703665,
    -0.8987940462991669,
    -0.8910065241883679,
    -0.8829475928589269,
    -0.8746197071393959,
    -0.8660254037844386,
    -0.8571673007021123,
    -0.8480480961564261,
    -0.838670567945424,
    -0.8290375725550418,
    -0.8191520442889918,
    -0.8090169943749476,
    -0.7986355100472928,
    -0.788010753606722,
    -0.7771459614569708,
    -0.766044443118978,
    -0.7547095802227721,
    -0.7431448254773942,
    -0.7313537016191706,
    -0.7193398003386511,
    -0.7071067811865477,
    -0.6946583704589973,
    -0.6819983600624986,
    -0.6691306063588581,
    -0.6560590289905073,
    -0.6427876096865395,
    -0.6293203910498378,
    -0.6156614753256581,
    -0.6018150231520483,
    -0.5877852522924732,
    -0.5735764363510464,
    -0.5591929034707466,
    -0.544639035015027,
    -0.529919264233205,
    -0.5150380749100545,
    -0.5000000000000004,
    -0.48480962024633684,
    -0.46947156278589075,
    -0.4539904997395469,
    -0.43837114678907774,
    -0.42261826174069916,
    -0.4067366430758001,
    -0.3907311284892738,
    -0.3746065934159123,
    -0.3583679495453007,
    -0.34202014332566855,
    -0.32556815445715664,
    -0.30901699437494756,
    -0.2923717047227371,
    -0.2756373558169989,
    -0.25881904510252063,
    -0.24192189559966779,
    -0.22495105434386525,
    -0.2079116908177598,
    -0.1908089953765446,
    -0.17364817766693033,
    -0.15643446504023104,
    -0.13917310096006583,
    -0.12186934340514717,
    -0.10452846326765336,
    -0.08715574274765825,
    -0.06975647374412558,
    -0.052335956242944306,
    -0.03489949670250076,
    -0.017452406437283498,
    0.0,
    0.01745240643728313,
    0.03489949670250128,
    0.052335956242943946,
    0.06975647374412522,
    0.08715574274765789,
    0.10452846326765299,
    0.12186934340514768,
    0.13917310096006547,
    0.15643446504023067,
    0.17364817766692997,
    0.1908089953765451,
    0.20791169081775943,
    0.22495105434386492,
    0.24192189559966745,
    0.2588190451025203,
    0.2756373558169994,
    0.2923717047227367,
    0.30901699437494723,
    0.3255681544571563,
    0.342020143325669,
    0.3583679495453004,
    0.37460659341591196,
    0.3907311284892735,
    0.40673664307579976,
    0.4226182617406996,
    0.4383711467890774,
    0.45399049973954664,
    0.4694715627858904,
    0.4848096202463373,
    0.5000000000000001,
    0.5150380749100542,
    0.5299192642332047,
    0.5446390350150266,
    0.559192903470747,
    0.573576436351046,
    0.5877852522924729,
    0.6018150231520479,
    0.6156614753256585,
    0.6293203910498375,
    0.6427876096865393,
    0.656059028990507,
    0.6691306063588578,
    0.6819983600624986,
    0.6946583704589973,
    0.7071067811865474,
    0.7193398003386509,
    0.7313537016191707,
    0.7431448254773942,
    0.7547095802227719,
    0.7660444431189778,
    0.7771459614569706,
    0.788010753606722,
    0.7986355100472928,
    0.8090169943749473,
    0.8191520442889916,
    0.8290375725550418,
    0.838670567945424,
    0.848048096156426,
    0.8571673007021121,
    0.8660254037844384,
    0.8746197071393959,
    0.8829475928589269,
    0.8910065241883678,
    0.8987940462991668,
    0.90630778703665,
    0.913545457642601,
    0.9205048534524403,
    0.9271838545667873,
    0.9335804264972015,
    0.9396926207859084,
    0.9455185755993168,
    0.9510565162951535,
    0.9563047559630353,
    0.9612616959383189,
    0.9659258262890683,
    0.9702957262759965,
    0.9743700647852351,
    0.9781476007338056,
    0.981627183447664,
    0.984807753012208,
    0.9876883405951377,
    0.9902680687415703,
    0.9925461516413221,
    0.9945218953682733,
    0.9961946980917455,
    0.9975640502598242,
    0.9986295347545738,
    0.9993908270190958,
    0.9998476951563913,
];

/// Sinus de chaque direction, en regard de [`COS`] : `SIN[d]` vaut le sinus de `d` degrés.
pub const SIN: [f64; NB_DIRECTIONS] = [
    0.0,
    0.01745240643728351,
    0.03489949670250097,
    0.052335956242943835,
    0.0697564737441253,
    0.08715574274765817,
    0.10452846326765347,
    0.12186934340514748,
    0.13917310096006544,
    0.15643446504023087,
    0.17364817766693033,
    0.1908089953765448,
    0.20791169081775934,
    0.224951054343865,
    0.24192189559966773,
    0.25881904510252074,
    0.27563735581699916,
    0.29237170472273677,
    0.3090169943749474,
    0.3255681544571567,
    0.3420201433256687,
    0.35836794954530027,
    0.374606593415912,
    0.39073112848927377,
    0.4067366430758002,
    0.42261826174069944,
    0.4383711467890774,
    0.45399049973954675,
    0.4694715627858908,
    0.48480962024633706,
    0.49999999999999994,
    0.5150380749100542,
    0.5299192642332049,
    0.5446390350150271,
    0.5591929034707469,
    0.573576436351046,
    0.5877852522924731,
    0.6018150231520483,
    0.6156614753256583,
    0.6293203910498374,
    0.6427876096865393,
    0.6560590289905073,
    0.6691306063588582,
    0.6819983600624985,
    0.6946583704589973,
    0.7071067811865475,
    0.7193398003386511,
    0.7313537016191705,
    0.7431448254773942,
    0.754709580222772,
    0.766044443118978,
    0.7771459614569709,
    0.788010753606722,
    0.7986355100472928,
    0.8090169943749475,
    0.8191520442889918,
    0.8290375725550417,
    0.838670567945424,
    0.848048096156426,
    0.8571673007021123,
    0.8660254037844386,
    0.8746197071393957,
    0.8829475928589269,
    0.8910065241883678,
    0.898794046299167,
    0.9063077870366499,
    0.9135454576426009,
    0.9205048534524404,
    0.9271838545667874,
    0.9335804264972017,
    0.9396926207859083,
    0.9455185755993167,
    0.9510565162951535,
    0.9563047559630354,
    0.9612616959383189,
    0.9659258262890683,
    0.9702957262759965,
    0.9743700647852352,
    0.9781476007338056,
    0.981627183447664,
    0.984807753012208,
    0.9876883405951378,
    0.9902680687415704,
    0.992546151641322,
    0.9945218953682733,
    0.9961946980917455,
    0.9975640502598242,
    0.9986295347545738,
    0.9993908270190958,
    0.9998476951563913,
    1.0,
    0.9998476951563913,
    0.9993908270190958,
    0.9986295347545738,
    0.9975640502598242,
    0.9961946980917455,
    0.9945218953682733,
    0.9925461516413221,
    0.9902680687415704,
    0.9876883405951378,
    0.984807753012208,
    0.981627183447664,
    0.9781476007338057,
    0.9743700647852352,
    0.9702957262759965,
    0.9659258262890683,
    0.9612616959383189,
    0.9563047559630355,
    0.9510565162951536,
    0.9455185755993168,
    0.9396926207859084,
    0.9335804264972017,
    0.9271838545667874,
    0.9205048534524403,
    0.9135454576426009,
    0.90630778703665,
    0.8987940462991669,
    0.8910065241883679,
    0.8829475928589269,
    0.8746197071393959,
    0.8660254037844387,
    0.8571673007021123,
    0.8480480961564261,
    0.8386705679454239,
    0.8290375725550417,
    0.8191520442889917,
    0.8090169943749475,
    0.7986355100472927,
    0.788010753606722,
    0.777145961456971,
    0.766044443118978,
    0.7547095802227721,
    0.7431448254773942,
    0.7313537016191706,
    0.7193398003386511,
    std::f64::consts::FRAC_1_SQRT_2,
    0.6946583704589971,
    0.6819983600624986,
    0.6691306063588583,
    0.6560590289905073,
    0.6427876096865395,
    0.6293203910498374,
    0.6156614753256584,
    0.6018150231520482,
    0.5877852522924732,
    0.5735764363510459,
    0.5591929034707469,
    0.5446390350150273,
    0.5299192642332049,
    0.5150380749100544,
    0.49999999999999994,
    0.48480962024633717,
    0.4694715627858907,
    0.45399049973954686,
    0.4383711467890773,
    0.4226182617406995,
    0.40673664307580043,
    0.39073112848927377,
    0.37460659341591224,
    0.3583679495453002,
    0.3420201433256689,
    0.3255681544571566,
    0.3090169943749475,
    0.2923717047227366,
    0.2756373558169992,
    0.258819045102521,
    0.24192189559966773,
    0.2249510543438652,
    0.20791169081775931,
    0.19080899537654497,
    0.17364817766693028,
    0.15643446504023098,
    0.13917310096006533,
    0.12186934340514755,
    0.10452846326765373,
    0.0871557427476582,
    0.06975647374412552,
    0.05233595624294381,
    0.03489949670250114,
    0.01745240643728344,
    0.0,
    -0.017452406437283637,
    -0.0348994967025009,
    -0.052335956242943564,
    -0.06975647374412527,
    -0.08715574274765794,
    -0.1045284632676535,
    -0.12186934340514731,
    -0.13917310096006552,
    -0.15643446504023073,
    -0.17364817766693047,
    -0.19080899537654472,
    -0.2079116908177595,
    -0.22495105434386498,
    -0.2419218955996675,
    -0.2588190451025208,
    -0.275637355816999,
    -0.29237170472273677,
    -0.3090169943749473,
    -0.32556815445715676,
    -0.34202014332566866,
    -0.35836794954530043,
    -0.374606593415912,
    -0.39073112848927355,
    -0.4067366430758002,
    -0.4226182617406993,
    -0.43837114678907746,
    -0.4539904997395467,
    -0.46947156278589086,
    -0.48480962024633695,
    -0.5000000000000001,
    -0.5150380749100542,
    -0.5299192642332048,
    -0.5446390350150271,
    -0.5591929034707467,
    -0.5735764363510462,
    -0.587785252292473,
    -0.6018150231520484,
    -0.6156614753256582,
    -0.6293203910498376,
    -0.6427876096865393,
    -0.656059028990507,
    -0.6691306063588582,
    -0.6819983600624984,
    -0.6946583704589974,
    -0.7071067811865475,
    -0.7193398003386512,
    -0.7313537016191705,
    -0.7431448254773944,
    -0.754709580222772,
    -0.7660444431189779,
    -0.7771459614569706,
    -0.7880107536067221,
    -0.7986355100472928,
    -0.8090169943749473,
    -0.8191520442889916,
    -0.8290375725550418,
    -0.838670567945424,
    -0.848048096156426,
    -0.8571673007021121,
    -0.8660254037844384,
    -0.874619707139396,
    -0.882947592858927,
    -0.8910065241883678,
    -0.8987940462991668,
    -0.90630778703665,
    -0.913545457642601,
    -0.9205048534524403,
    -0.9271838545667873,
    -0.9335804264972016,
    -0.9396926207859084,
    -0.9455185755993168,
    -0.9510565162951535,
    -0.9563047559630353,
    -0.961261695938319,
    -0.9659258262890683,
    -0.9702957262759965,
    -0.9743700647852351,
    -0.9781476007338056,
    -0.981627183447664,
    -0.984807753012208,
    -0.9876883405951377,
    -0.9902680687415703,
    -0.9925461516413221,
    -0.9945218953682734,
    -0.9961946980917455,
    -0.9975640502598242,
    -0.9986295347545738,
    -0.9993908270190958,
    -0.9998476951563913,
    -1.0,
    -0.9998476951563913,
    -0.9993908270190958,
    -0.9986295347545738,
    -0.9975640502598243,
    -0.9961946980917455,
    -0.9945218953682734,
    -0.992546151641322,
    -0.9902680687415704,
    -0.9876883405951378,
    -0.9848077530122081,
    -0.9816271834476639,
    -0.9781476007338056,
    -0.9743700647852352,
    -0.9702957262759966,
    -0.9659258262890684,
    -0.9612616959383188,
    -0.9563047559630354,
    -0.9510565162951536,
    -0.945518575599317,
    -0.9396926207859083,
    -0.9335804264972017,
    -0.9271838545667874,
    -0.9205048534524405,
    -0.9135454576426011,
    -0.9063077870366499,
    -0.898794046299167,
    -0.891006524188368,
    -0.8829475928589271,
    -0.8746197071393956,
    -0.8660254037844386,
    -0.8571673007021123,
    -0.8480480961564262,
    -0.8386705679454243,
    -0.8290375725550416,
    -0.8191520442889918,
    -0.8090169943749476,
    -0.798635510047293,
    -0.7880107536067218,
    -0.7771459614569708,
    -0.7660444431189781,
    -0.7547095802227722,
    -0.7431448254773946,
    -0.7313537016191703,
    -0.7193398003386512,
    -0.7071067811865477,
    -0.6946583704589976,
    -0.6819983600624983,
    -0.6691306063588581,
    -0.6560590289905074,
    -0.6427876096865396,
    -0.6293203910498378,
    -0.6156614753256582,
    -0.6018150231520483,
    -0.5877852522924734,
    -0.5735764363510465,
    -0.5591929034707466,
    -0.544639035015027,
    -0.529919264233205,
    -0.5150380749100545,
    -0.5000000000000004,
    -0.4848096202463369,
    -0.4694715627858908,
    -0.45399049973954697,
    -0.4383711467890778,
    -0.4226182617406992,
    -0.40673664307580015,
    -0.3907311284892739,
    -0.37460659341591235,
    -0.35836794954530077,
    -0.3420201433256686,
    -0.3255681544571567,
    -0.3090169943749476,
    -0.29237170472273716,
    -0.27563735581699894,
    -0.2588190451025207,
    -0.24192189559966787,
    -0.22495105434386534,
    -0.20791169081775987,
    -0.19080899537654467,
    -0.1736481776669304,
    -0.15643446504023112,
    -0.13917310096006588,
    -0.12186934340514723,
    -0.10452846326765342,
    -0.08715574274765832,
    -0.06975647374412564,
    -0.05233595624294437,
    -0.034899496702500823,
    -0.01745240643728356,
];

/// Les paramètres de la croissance. Ils ne changent pas pendant la vie d'une [`Colonie`].
///
/// Contrat : `pas` est fini et strictement positif ; `intervalle` est supérieur ou égal à 1, ou vaut
/// `f64::INFINITY` (aucune ramification) ; `angle_ramification` et `amplitude_marche` sont inférieurs à
/// [`NB_DIRECTIONS`]. Les fonctions qui reçoivent des paramètres hors contrat paniquent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parametres {
    /// Longueur dont chaque apex avance à chaque pas de temps, dans l'unité du plan.
    pub pas: f64,
    /// Intervalle moyen de ramification, en pas de temps : chaque apex se ramifie à chaque pas avec la probabilité
    /// `1 / intervalle`, indépendamment des autres pas. Quand la ramification est possible, la première ramification
    /// de la colonie est garantie dans ses 30 premiers pas. `f64::INFINITY` supprime toute ramification.
    pub intervalle: f64,
    /// Angle, en degrés (indices de la table), entre la direction de l'apex et celle de la branche neuve ; la
    /// branche part d'un côté ou de l'autre, à parts égales.
    pub angle_ramification: u32,
    /// Amplitude de la marche persistante, en degrés : à chaque pas, la direction tourne d'un entier tiré
    /// uniformément dans `-amplitude_marche..=amplitude_marche`. Zéro donne des apex qui vont tout droit.
    pub amplitude_marche: u32,
}

impl Parametres {
    /// Probabilité qu'un apex se ramifie à un pas donné : `1 / intervalle`, et `0.0` quand `intervalle` est infini.
    ///
    /// # Panics
    ///
    /// Si les paramètres sont hors contrat.
    pub fn probabilite_ramification(&self) -> f64 {
        self.verifier();
        if self.intervalle.is_infinite() { 0.0 } else { 1.0 / self.intervalle }
    }

    /// Vérifie le contrat décrit sur [`Parametres`], et panique s'il est rompu.
    fn verifier(&self) {
        assert!(self.pas.is_finite() && self.pas > 0.0, "pas doit être fini et strictement positif : {}", self.pas);
        assert!(
            self.intervalle >= 1.0 || self.intervalle == f64::INFINITY,
            "intervalle doit être au moins 1, ou infini : {}",
            self.intervalle
        );
        assert!(
            (self.angle_ramification as usize) < NB_DIRECTIONS,
            "angle_ramification doit être inférieur à {NB_DIRECTIONS} : {}",
            self.angle_ramification
        );
        assert!(
            (self.amplitude_marche as usize) < NB_DIRECTIONS,
            "amplitude_marche doit être inférieure à {NB_DIRECTIONS} : {}",
            self.amplitude_marche
        );
    }
}

/// Un apex : la pointe vivante d'une hyphe.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Apex {
    /// Identifiant, unique dans la colonie : le premier apex a l'identifiant 0, puis les identifiants sont
    /// distribués dans l'ordre de naissance, sans trou.
    pub id: u32,
    /// Abscisse de la pointe.
    pub x: f64,
    /// Ordonnée de la pointe.
    pub y: f64,
    /// Direction courante : un indice dans `0..NB_DIRECTIONS`, à lire dans [`COS`] et [`SIN`].
    pub direction: usize,
}

/// Un segment d'hyphe : ce qu'un apex laisse derrière lui en avançant d'un pas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    /// Identifiant de l'[`Apex`] qui a laissé ce segment.
    pub apex: u32,
    /// Numéro du pas de temps (à partir de 1) pendant lequel le segment a été laissé.
    pub temps: u32,
    /// Abscisse du début, là où l'apex se trouvait avant d'avancer.
    pub x0: f64,
    /// Ordonnée du début.
    pub y0: f64,
    /// Abscisse de la fin, là où l'apex se trouve après avoir avancé.
    pub x1: f64,
    /// Ordonnée de la fin.
    pub y1: f64,
}

impl Segment {
    /// Écriture canonique : [`OCTETS_PAR_SEGMENT`] octets, sans remplissage, tous les nombres en petit-boutiste :
    /// `apex` (`u32`), `temps` (`u32`), puis `x0`, `y0`, `x1`, `y1` (chacun les 8 octets de `f64::to_le_bytes`).
    /// Deux segments ont la même écriture si et seulement si leurs champs sont identiques bit à bit.
    pub fn octets(&self) -> [u8; OCTETS_PAR_SEGMENT] {
        let mut octets = [0u8; OCTETS_PAR_SEGMENT];
        octets[0..4].copy_from_slice(&self.apex.to_le_bytes());
        octets[4..8].copy_from_slice(&self.temps.to_le_bytes());
        octets[8..16].copy_from_slice(&self.x0.to_le_bytes());
        octets[16..24].copy_from_slice(&self.y0.to_le_bytes());
        octets[24..32].copy_from_slice(&self.x1.to_le_bytes());
        octets[32..40].copy_from_slice(&self.y1.to_le_bytes());
        octets
    }

    /// Longueur du segment, `sqrt(dx² + dy²)`.
    pub fn longueur(&self) -> f64 {
        let dx = self.x1 - self.x0;
        let dy = self.y1 - self.y0;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Une colonie : les apex vivants, les segments déjà laissés et le générateur pseudo-aléatoire.
///
/// Un état entièrement déterminé par les paramètres, le centre, la graine et le nombre de pas joués : deux colonies
/// construites pareil et avancées pareil ont exactement les mêmes segments, à l'octet de leur écriture canonique.
#[derive(Clone, Debug)]
pub struct Colonie {
    parametres: Parametres,
    apex: Vec<Apex>,
    segments: Vec<Segment>,
    temps: u32,
    nb_ramifications: u64,
    alea: Alea,
}

impl Colonie {
    /// Crée une colonie à l'instant 0 depuis une spore posée en `centre` (abscisse, ordonnée) : un seul apex,
    /// d'identifiant 0, dont la direction est tirée uniformément dans `0..NB_DIRECTIONS` avec le générateur
    /// `Alea::new(graine)`. Aucun segment, aucune ramification.
    ///
    /// # Panics
    ///
    /// Si les paramètres sont hors contrat.
    pub fn depuis_spore(centre: (f64, f64), parametres: Parametres, graine: u64) -> Colonie {
        parametres.verifier();
        let mut alea = Alea::new(graine);
        let direction = (alea.suivant() % NB_DIRECTIONS as u64) as usize;
        let apex = Apex { id: 0, x: centre.0, y: centre.1, direction };
        Colonie { parametres, apex: vec![apex], segments: Vec::new(), temps: 0, nb_ramifications: 0, alea }
    }

    /// Avance la colonie d'un pas de temps, selon le modèle décrit en tête de module. Les apex sont mis à jour dans
    /// l'ordre de leur indice dans [`Colonie::apex`] ; les apex nés pendant le pas sont ajoutés à la fin, sans
    /// avancer avant le pas suivant. Chaque apex vivant laisse exactement un segment par pas, ajouté à la fin de
    /// [`Colonie::segments`].
    pub fn avancer(&mut self) {
        let probabilite = self.parametres.probabilite_ramification();
        let amplitude = self.parametres.amplitude_marche as u64;
        let angle_ramification = self.parametres.angle_ramification as i64;
        let nb_apex_au_debut = self.apex.len();
        self.temps += 1;
        for i in 0..nb_apex_au_debut {
            let mut apex = self.apex[i];

            // 1. Tourner d'un angle entier tiré uniformément dans `-amplitude..=amplitude`.
            let delta = (self.alea.suivant() % (2 * amplitude + 1)) as i64 - amplitude as i64;
            apex.direction = (apex.direction as i64 + delta).rem_euclid(NB_DIRECTIONS as i64) as usize;

            // 2. Avancer d'un pas et laisser un segment.
            let direction = apex.direction;
            let (x0, y0) = (apex.x, apex.y);
            let (x1, y1) = (x0 + self.parametres.pas * COS[direction], y0 + self.parametres.pas * SIN[direction]);
            apex.x = x1;
            apex.y = y1;
            self.apex[i] = apex;
            self.segments.push(Segment { apex: apex.id, temps: self.temps, x0, y0, x1, y1 });

            // 3. Se ramifier : un nouvel apex naît à la nouvelle position, d'un côté ou de l'autre.
            if probabilite > 0.0 && self.alea.suivant_f64() < probabilite {
                let cote = self.alea.suivant() % 2;
                let decalage = if cote == 0 { angle_ramification } else { -angle_ramification };
                let direction = (direction as i64 + decalage).rem_euclid(NB_DIRECTIONS as i64) as usize;
                let id = self.apex.len() as u32;
                self.apex.push(Apex { id, x: x1, y: y1, direction });
                self.nb_ramifications += 1;
            }
        }
    }

    /// Avance la colonie de `n` pas de temps : exactement le même état que `n` appels à [`Colonie::avancer`].
    pub fn avancer_n(&mut self, n: u32) {
        for _ in 0..n {
            self.avancer();
        }
    }

    /// Les paramètres de la colonie.
    pub fn parametres(&self) -> &Parametres {
        &self.parametres
    }

    /// Les apex vivants, dans l'ordre de leur naissance (donc de leur identifiant). Aucun apex ne meurt : la
    /// tranche ne fait que s'allonger.
    pub fn apex(&self) -> &[Apex] {
        &self.apex
    }

    /// Tous les segments laissés depuis la création, dans l'ordre où ils ont été laissés.
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// Nombre de pas de temps joués depuis la création.
    pub fn temps(&self) -> u32 {
        self.temps
    }

    /// Nombre de ramifications survenues depuis la création : c'est aussi le nombre d'apex moins un.
    pub fn nb_ramifications(&self) -> u64 {
        self.nb_ramifications
    }

    /// Écriture canonique des segments : la concaténation, dans l'ordre de [`Colonie::segments`], de
    /// [`Segment::octets`]. Sa longueur est `OCTETS_PAR_SEGMENT × segments().len()`. C'est elle qu'on compare « à
    /// l'octet ».
    pub fn octets(&self) -> Vec<u8> {
        let mut octets = Vec::with_capacity(OCTETS_PAR_SEGMENT * self.segments.len());
        for segment in &self.segments {
            octets.extend_from_slice(&segment.octets());
        }
        octets
    }
}
