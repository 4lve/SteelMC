use std::str::FromStr;

/// A locale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Locale {
    AfZa,
    ArSa,
    AstEs,
    AzAz,
    BaRu,
    Bar,
    BeBy,
    BgBg,
    BrFr,
    Brb,
    BsBa,
    CaEs,
    CsCz,
    CyGb,
    DaDk,
    DeAt,
    DeCh,
    DeDe,
    ElGr,
    EnAu,
    EnCa,
    EnGb,
    EnNz,
    EnPt,
    EnUd,
    EnUs,
    Enp,
    Enws,
    EoUy,
    EsAr,
    EsCl,
    EsEc,
    EsEs,
    EsMx,
    EsUy,
    EsVe,
    Esan,
    EtEe,
    EuEs,
    FaIr,
    FiFi,
    FilPh,
    FoFo,
    FrCa,
    FrFr,
    FraDe,
    FurIt,
    FyNl,
    GaIe,
    GdGb,
    GlEs,
    HawUs,
    HeIl,
    HiIn,
    HrHr,
    HuHu,
    HyAm,
    IdId,
    IgNg,
    IoEn,
    IsIs,
    Isv,
    ItIt,
    JaJp,
    JboEn,
    KaGe,
    KkKz,
    KnIn,
    KoKr,
    Ksh,
    KwGb,
    LaLa,
    LbLu,
    LiLi,
    Lmo,
    LoLa,
    LolUs,
    LtLt,
    LvLv,
    Lzh,
    MkMk,
    MnMn,
    MsMy,
    MtMt,
    Nah,
    NdsDe,
    NlBe,
    NlNl,
    NnNo,
    NoNo,
    OcFr,
    Ovd,
    PlPl,
    PtBr,
    PtPt,
    QyaAa,
    RoRo,
    Rpr,
    RuRu,
    RyUa,
    SahSah,
    SeNo,
    SkSk,
    SlSi,
    SoSo,
    SqAl,
    SrCs,
    SrSp,
    SvSe,
    Sxu,
    Szl,
    TaIn,
    ThTh,
    TlPh,
    TlhAa,
    Tok,
    TrTr,
    TtRu,
    UkUa,
    ValEs,
    VecIt,
    ViVn,
    YiDe,
    YoNg,
    ZhCn,
    ZhHk,
    ZhTw,
    ZlmArab,
}

#[allow(missing_docs)]
impl FromStr for Locale {
    type Err = ();

    #[allow(clippy::too_many_lines, clippy::match_same_arms)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "af_za" => Ok(Locale::AfZa),       // Afrikaans (Suid-Afrika)
            "ar_sa" => Ok(Locale::ArSa),       // Arabic
            "ast_es" => Ok(Locale::AstEs),     // Asturian
            "az_az" => Ok(Locale::AzAz),       // Azerbaijani
            "ba_ru" => Ok(Locale::BaRu),       // Bashkir
            "bar" => Ok(Locale::Bar),          // Bavarian
            "be_by" => Ok(Locale::BeBy),       // Belarusian
            "bg_bg" => Ok(Locale::BgBg),       // Bulgarian
            "br_fr" => Ok(Locale::BrFr),       // Breton
            "brb" => Ok(Locale::Brb),          // Brabantian
            "bs_ba" => Ok(Locale::BsBa),       // Bosnian
            "ca_es" => Ok(Locale::CaEs),       // Catalan
            "cs_cz" => Ok(Locale::CsCz),       // Czech
            "cy_gb" => Ok(Locale::CyGb),       // Welsh
            "da_dk" => Ok(Locale::DaDk),       // Danish
            "de_at" => Ok(Locale::DeAt),       // Austrian German
            "de_ch" => Ok(Locale::DeCh),       // Swiss German
            "de_de" => Ok(Locale::DeDe),       // German
            "el_gr" => Ok(Locale::ElGr),       // Greek
            "en_au" => Ok(Locale::EnAu),       // Australian English
            "en_ca" => Ok(Locale::EnCa),       // Canadian English
            "en_gb" => Ok(Locale::EnGb),       // British English
            "en_nz" => Ok(Locale::EnNz),       // New Zealand English
            "en_pt" => Ok(Locale::EnPt),       // Pirate English
            "en_ud" => Ok(Locale::EnUd),       // Upside down British English
            "en_us" => Ok(Locale::EnUs),       // American English
            "enp" => Ok(Locale::Enp),          // Modern English minus borrowed words
            "enws" => Ok(Locale::Enws),        // Early Modern English
            "eo_uy" => Ok(Locale::EoUy),       // Esperanto
            "es_ar" => Ok(Locale::EsAr),       // Argentinian Spanish
            "es_cl" => Ok(Locale::EsCl),       // Chilean Spanish
            "es_ec" => Ok(Locale::EsEc),       // Ecuadorian Spanish
            "es_es" => Ok(Locale::EsEs),       // European Spanish
            "es_mx" => Ok(Locale::EsMx),       // Mexican Spanish
            "es_uy" => Ok(Locale::EsUy),       // Uruguayan Spanish
            "es_ve" => Ok(Locale::EsVe),       // Venezuelan Spanish
            "esan" => Ok(Locale::Esan),        // Andalusian
            "et_ee" => Ok(Locale::EtEe),       // Estonian
            "eu_es" => Ok(Locale::EuEs),       // Basque
            "fa_ir" => Ok(Locale::FaIr),       // Persian
            "fi_fi" => Ok(Locale::FiFi),       // Finnish
            "fil_ph" => Ok(Locale::FilPh),     // Filipino
            "fo_fo" => Ok(Locale::FoFo),       // Faroese
            "fr_ca" => Ok(Locale::FrCa),       // Canadian French
            "fr_fr" => Ok(Locale::FrFr),       // European French
            "fra_de" => Ok(Locale::FraDe),     // East Franconian
            "fur_it" => Ok(Locale::FurIt),     // Friulian
            "fy_nl" => Ok(Locale::FyNl),       // Frisian
            "ga_ie" => Ok(Locale::GaIe),       // Irish
            "gd_gb" => Ok(Locale::GdGb),       // Scottish Gaelic
            "gl_es" => Ok(Locale::GlEs),       // Galician
            "haw_us" => Ok(Locale::HawUs),     // Hawaiian
            "he_il" => Ok(Locale::HeIl),       // Hebrew
            "hi_in" => Ok(Locale::HiIn),       // Hindi
            "hr_hr" => Ok(Locale::HrHr),       // Croatian
            "hu_hu" => Ok(Locale::HuHu),       // Hungarian
            "hy_am" => Ok(Locale::HyAm),       // Armenian
            "id_id" => Ok(Locale::IdId),       // Indonesian
            "ig_ng" => Ok(Locale::IgNg),       // Igbo
            "io_en" => Ok(Locale::IoEn),       // Ido
            "is_is" => Ok(Locale::IsIs),       // Icelandic
            "isv" => Ok(Locale::Isv),          // Interslavic
            "it_it" => Ok(Locale::ItIt),       // Italian
            "ja_jp" => Ok(Locale::JaJp),       // Japanese
            "jbo_en" => Ok(Locale::JboEn),     // Lojban
            "ka_ge" => Ok(Locale::KaGe),       // Georgian
            "kk_kz" => Ok(Locale::KkKz),       // Kazakh
            "kn_in" => Ok(Locale::KnIn),       // Kannada
            "ko_kr" => Ok(Locale::KoKr),       // Korean
            "ksh" => Ok(Locale::Ksh),          // Kölsch/Ripuarian
            "kw_gb" => Ok(Locale::KwGb),       // Cornish
            "la_la" => Ok(Locale::LaLa),       // Latin
            "lb_lu" => Ok(Locale::LbLu),       // Luxembourgish
            "li_li" => Ok(Locale::LiLi),       // Limburgish
            "lmo" => Ok(Locale::Lmo),          // Lombard
            "lo_la" => Ok(Locale::LoLa),       // Lao
            "lol_us" => Ok(Locale::LolUs),     // LOLCAT
            "lt_lt" => Ok(Locale::LtLt),       // Lithuanian
            "lv_lv" => Ok(Locale::LvLv),       // Latvian
            "lzh" => Ok(Locale::Lzh),          // Classical Chinese
            "mk_mk" => Ok(Locale::MkMk),       // Macedonian
            "mn_mn" => Ok(Locale::MnMn),       // Mongolian
            "ms_my" => Ok(Locale::MsMy),       // Malay
            "mt_mt" => Ok(Locale::MtMt),       // Maltese
            "nah" => Ok(Locale::Nah),          // Nahuatl
            "nds_de" => Ok(Locale::NdsDe),     // Low German
            "nl_be" => Ok(Locale::NlBe),       // Dutch, Flemish
            "nl_nl" => Ok(Locale::NlNl),       // Dutch
            "nn_no" => Ok(Locale::NnNo),       // Norwegian Nynorsk
            "no_no" => Ok(Locale::NoNo),       // Norwegian Bokmål
            "oc_fr" => Ok(Locale::OcFr),       // Occitan
            "ovd" => Ok(Locale::Ovd),          // Elfdalian
            "pl_pl" => Ok(Locale::PlPl),       // Polish
            "pt_br" => Ok(Locale::PtBr),       // Brazilian Portuguese
            "pt_pt" => Ok(Locale::PtPt),       // European Portuguese
            "qya_aa" => Ok(Locale::QyaAa),     // Quenya (Form of Elvish from LOTR)
            "ro_ro" => Ok(Locale::RoRo),       // Romanian
            "rpr" => Ok(Locale::Rpr),          // Russian (Pre-revolutionary)
            "ru_ru" => Ok(Locale::RuRu),       // Russian
            "ry_ua" => Ok(Locale::RyUa),       // Rusyn
            "sah_sah" => Ok(Locale::SahSah),   // Yakut
            "se_no" => Ok(Locale::SeNo),       // Northern Sami
            "sk_sk" => Ok(Locale::SkSk),       // Slovak
            "sl_si" => Ok(Locale::SlSi),       // Slovenian
            "so_so" => Ok(Locale::SoSo),       // Somali
            "sq_al" => Ok(Locale::SqAl),       // Albanian
            "sr_cs" => Ok(Locale::SrCs),       // Serbian (Latin)
            "sr_sp" => Ok(Locale::SrSp),       // Serbian (Cyrillic)
            "sv_se" => Ok(Locale::SvSe),       // Swedish
            "sxu" => Ok(Locale::Sxu),          // Upper Saxon German
            "szl" => Ok(Locale::Szl),          // Silesian
            "ta_in" => Ok(Locale::TaIn),       // Tamil
            "th_th" => Ok(Locale::ThTh),       // Thai
            "tl_ph" => Ok(Locale::TlPh),       // Tagalog
            "tlh_aa" => Ok(Locale::TlhAa),     // Klingon
            "tok" => Ok(Locale::Tok),          // Toki Pona
            "tr_tr" => Ok(Locale::TrTr),       // Turkish
            "tt_ru" => Ok(Locale::TtRu),       // Tatar
            "uk_ua" => Ok(Locale::UkUa),       // Ukrainian
            "val_es" => Ok(Locale::ValEs),     // Valencian
            "vec_it" => Ok(Locale::VecIt),     // Venetian
            "vi_vn" => Ok(Locale::ViVn),       // Vietnamese
            "yi_de" => Ok(Locale::YiDe),       // Yiddish
            "yo_ng" => Ok(Locale::YoNg),       // Yoruba
            "zh_cn" => Ok(Locale::ZhCn),       // Chinese Simplified (China; Mandarin)
            "zh_hk" => Ok(Locale::ZhHk),       // Chinese Traditional (Hong Kong; Mix)
            "zh_tw" => Ok(Locale::ZhTw),       // Chinese Traditional (Taiwan; Mandarin)
            "zlm_arab" => Ok(Locale::ZlmArab), // Malay (Jawi)
            _ => Ok(Locale::EnUs),             // Default to English (US) if not found
        }
    }
}
