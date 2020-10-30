use super::CodePageInfo;
use ::console;
use ::errors::WinResult;

enumeration! {
	/// Represents a console code page.
	#[repr(u16)]
	#[allow(non_camel_case_types)]
	CodePage<u16> {
		__DEFAULT__ = 65535,
		/// Default value.
		None = 0,
		/// IBM EBCDIC US-Canada.
		IBM037 = 037,
		/// OEM United States.
		IBM437 = 437,
		/// IBM EBCDIC International.
		IBM500 = 500,
		/// Arabic (ASMO 708).
		ASMO_708 = 708,
		/// Arabic (ASMO-449+, BCON V4).
		ASMO_709 = 709,
		/// Arabic - Transparent Arabic.
		ASMO_710 = 710,
		/// Arabic (Transparent ASMO); Arabic (DOS).
		DOS_720 = 720,
		/// OEM Greek (formerly 437G); Greek (DOS).
		ibm737 = 737,
		/// OEM Baltic; Baltic (DOS).
		ibm775 = 775,
		/// OEM Multilingual Latin 1; Western European (DOS).
		ibm850 = 850,
		/// OEM Latin 2; Central European (DOS).
		ibm852 = 852,
		/// OEM Cyrillic (primarily Russian).
		IBM855 = 855,
		/// OEM Turkish; Turkish (DOS).
		ibm857 = 857,
		/// OEM Multilingual Latin 1 + Euro symbol.
		IBM00858 = 858,
		/// OEM Portuguese; Portuguese (DOS).
		IBM860 = 860,
		/// OEM Icelandic; Icelandic (DOS).
		ibm861 = 861,
		/// OEM Hebrew; Hebrew (DOS).
		DOS_862 = 862,
		/// OEM French Canadian; French Canadian (DOS).
		IBM863 = 863,
		/// OEM Arabic; Arabic (864).
		IBM864 = 864,
		/// OEM Nordic; Nordic (DOS).
		IBM865 = 865,
		/// OEM Russian; Cyrillic (DOS).
		cp866 = 866,
		/// OEM Modern Greek; Greek, Modern (DOS).
		ibm869 = 869,
		/// IBM EBCDIC Multilingual/ROECE (Latin 2); IBM EBCDIC Multilingual Latin 2.
		IBM870 = 870,
		/// ANSI/OEM Thai (ISO 8859-11); Thai (Windows).
		windows_874 = 874,
		/// IBM EBCDIC Greek Modern.
		cp875 = 875,
		/// ANSI/OEM Japanese; Japanese (Shift-JIS).
		shift_jis = 932,
		/// ANSI/OEM Simplified Chinese (PRC, Singapore); Chinese Simplified (GB2312).
		gb2312 = 936,
		/// ANSI/OEM Korean (Unified Hangul Code).
		ks_c_5601_1987 = 949,
		/// ANSI/OEM Traditional Chinese (Taiwan; Hong Kong SAR, PRC); Chinese Traditional (Big5).
		big5 = 950,
		/// IBM EBCDIC Turkish (Latin 5).
		IBM1026 = 1026,
		/// IBM EBCDIC Latin 1/Open System.
		IBM01047 = 1047,
		/// IBM EBCDIC US-Canada (037 + Euro symbol); IBM EBCDIC (US-Canada-Euro).
		IBM01140 = 1140,
		/// IBM EBCDIC Germany (20273 + Euro symbol); IBM EBCDIC (Germany-Euro).
		IBM01141 = 1141,
		/// IBM EBCDIC Denmark-Norway (20277 + Euro symbol); IBM EBCDIC (Denmark-Norway-Euro).
		IBM01142 = 1142,
		/// IBM EBCDIC Finland-Sweden (20278 + Euro symbol); IBM EBCDIC (Finland-Sweden-Euro).
		IBM01143 = 1143,
		/// IBM EBCDIC Italy (20280 + Euro symbol); IBM EBCDIC (Italy-Euro).
		IBM01144 = 1144,
		/// IBM EBCDIC Latin America-Spain (20284 + Euro symbol); IBM EBCDIC (Spain-Euro).
		IBM01145 = 1145,
		/// IBM EBCDIC United Kingdom (20285 + Euro symbol); IBM EBCDIC (UK-Euro).
		IBM01146 = 1146,
		/// IBM EBCDIC France (20297 + Euro symbol); IBM EBCDIC (France-Euro).
		IBM01147 = 1147,
		/// IBM EBCDIC International (500 + Euro symbol); IBM EBCDIC (International-Euro).
		IBM01148 = 1148,
		/// IBM EBCDIC Icelandic (20871 + Euro symbol); IBM EBCDIC (Icelandic-Euro).
		IBM01149 = 1149,
		/// Unicode UTF-16, little endian byte order (BMP of ISO 10646); available only to managed applications.
		utf_16 = 1200,
		/// Unicode UTF-16, big endian byte order; available only to managed applications.
		unicodeFFFE = 1201,
		/// ANSI Central European; Central European (Windows).
		windows_1250 = 1250,
		/// ANSI Cyrillic; Cyrillic (Windows).
		windows_1251 = 1251,
		/// ANSI Latin 1; Western European (Windows).
		windows_1252 = 1252,
		/// ANSI Greek; Greek (Windows).
		windows_1253 = 1253,
		/// ANSI Turkish; Turkish (Windows).
		windows_1254 = 1254,
		/// ANSI Hebrew; Hebrew (Windows).
		windows_1255 = 1255,
		/// ANSI Arabic; Arabic (Windows).
		windows_1256 = 1256,
		/// ANSI Baltic; Baltic (Windows).
		windows_1257 = 1257,
		/// ANSI/OEM Vietnamese; Vietnamese (Windows).
		windows_1258 = 1258,
		/// Korean (Johab).
		Johab = 1361,
		/// MAC Roman; Western European (Mac).
		macintosh = 10000,
		/// Japanese (Mac).
		x_mac_japanese = 10001,
		/// MAC Traditional Chinese (Big5); Chinese Traditional (Mac).
		x_mac_chinesetrad = 10002,
		/// Korean (Mac).
		x_mac_korean = 10003,
		/// Arabic (Mac).
		x_mac_arabic = 10004,
		/// Hebrew (Mac).
		x_mac_hebrew = 10005,
		/// Greek (Mac).
		x_mac_greek = 10006,
		/// Cyrillic (Mac).
		x_mac_cyrillic = 10007,
		/// MAC Simplified Chinese (GB 2312); Chinese Simplified (Mac).
		x_mac_chinesesimp = 10008,
		/// Romanian (Mac).
		x_mac_romanian = 10010,
		/// Ukrainian (Mac).
		x_mac_ukrainian = 10017,
		/// Thai (Mac).
		x_mac_thai = 10021,
		/// MAC Latin 2; Central European (Mac).
		x_mac_ce = 10029,
		/// Icelandic (Mac).
		x_mac_icelandic = 10079,
		/// Turkish (Mac).
		x_mac_turkish = 10081,
		/// Croatian (Mac).
		x_mac_croatian = 10082,
		/// Unicode UTF-32, little endian byte order; available only to managed applications.
		utf_32 = 12000,
		/// Unicode UTF-32, big endian byte order; available only to managed applications.
		utf_32BE = 12001,
		/// CNS Taiwan; Chinese Traditional (CNS).
		x_Chinese_CNS = 20000,
		/// TCA Taiwan.
		x_cp20001 = 20001,
		/// Eten Taiwan; Chinese Traditional (Eten).
		x_Chinese_Eten = 20002,
		/// IBM5550 Taiwan.
		x_cp20003 = 20003,
		/// TeleText Taiwan.
		x_cp20004 = 20004,
		/// Wang Taiwan.
		x_cp20005 = 20005,
		/// IA5 (IRV International Alphabet No. 5, 7-bit); Western European (IA5).
		x_IA5 = 20105,
		/// IA5 German (7-bit).
		x_IA5_German = 20106,
		/// IA5 Swedish (7-bit).
		x_IA5_Swedish = 20107,
		/// IA5 Norwegian (7-bit).
		x_IA5_Norwegian = 20108,
		/// US-ASCII (7-bit).
		us_ascii = 20127,
		/// T.61.
		x_cp20261 = 20261,
		/// ISO 6937 Non-Spacing Accent.
		x_cp20269 = 20269,
		/// IBM EBCDIC Germany.
		IBM273 = 20273,
		/// IBM EBCDIC Denmark-Norway.
		IBM277 = 20277,
		/// IBM EBCDIC Finland-Sweden.
		IBM278 = 20278,
		/// IBM EBCDIC Italy.
		IBM280 = 20280,
		/// IBM EBCDIC Latin America-Spain.
		IBM284 = 20284,
		/// IBM EBCDIC United Kingdom.
		IBM285 = 20285,
		/// IBM EBCDIC Japanese Katakana Extended.
		IBM290 = 20290,
		/// IBM EBCDIC France.
		IBM297 = 20297,
		/// IBM EBCDIC Arabic.
		IBM420 = 20420,
		/// IBM EBCDIC Greek.
		IBM423 = 20423,
		/// IBM EBCDIC Hebrew.
		IBM424 = 20424,
		/// IBM EBCDIC Korean Extended.
		x_EBCDIC_KoreanExtended = 20833,
		/// IBM EBCDIC Thai.
		IBM_Thai = 20838,
		/// Russian (KOI8-R); Cyrillic (KOI8-R).
		koi8_r = 20866,
		/// IBM EBCDIC Icelandic.
		IBM871 = 20871,
		/// IBM EBCDIC Cyrillic Russian.
		IBM880 = 20880,
		/// IBM EBCDIC Turkish.
		IBM905 = 20905,
		/// IBM EBCDIC Latin 1/Open System (1047 + Euro symbol).
		IBM00924 = 20924,
		/// Japanese (JIS 0208-1990 and 0212-1990).
		EUC_JP = 20932,
		/// Simplified Chinese (GB2312); Chinese Simplified (GB2312-80).
		x_cp20936 = 20936,
		/// Korean Wansung.
		x_cp20949 = 20949,
		/// IBM EBCDIC Cyrillic Serbian-Bulgarian.
		cp1025 = 21025,
		/// Ukrainian (KOI8-U); Cyrillic (KOI8-U).
		koi8_u = 21866,
		/// ISO 8859-1 Latin 1; Western European (ISO).
		iso_8859_1 = 28591,
		/// ISO 8859-2 Central European; Central European (ISO).
		iso_8859_2 = 28592,
		/// ISO 8859-3 Latin 3.
		iso_8859_3 = 28593,
		/// ISO 8859-4 Baltic.
		iso_8859_4 = 28594,
		/// ISO 8859-5 Cyrillic.
		iso_8859_5 = 28595,
		/// ISO 8859-6 Arabic.
		iso_8859_6 = 28596,
		/// ISO 8859-7 Greek.
		iso_8859_7 = 28597,
		/// ISO 8859-8 Hebrew; Hebrew (ISO-Visual).
		iso_8859_8 = 28598,
		/// ISO 8859-9 Turkish.
		iso_8859_9 = 28599,
		/// ISO 8859-13 Estonian.
		iso_8859_13 = 28603,
		/// ISO 8859-15 Latin 9.
		iso_8859_15 = 28605,
		/// Europa 3.
		x_Europa = 29001,
		/// ISO 8859-8 Hebrew; Hebrew (ISO-Logical).
		iso_8859_8_i = 38598,
		/// ISO 2022 Japanese with no halfwidth Katakana; Japanese (JIS).
		iso_2022_jp_1 = 50220,
		/// ISO 2022 Japanese with halfwidth Katakana; Japanese (JIS-Allow 1 byte Kana).
		csISO2022JP = 50221,
		/// ISO 2022 Japanese JIS X 0201-1989; Japanese (JIS-Allow 1 byte Kana - SO/SI).
		iso_2022_jp_2 = 50222,
		/// ISO 2022 Korean.
		iso_2022_kr = 50225,
		/// ISO 2022 Simplified Chinese; Chinese Simplified (ISO 2022).
		x_cp50227 = 50227,
		/// ISO 2022 Traditional Chinese.
		iso_2022_ch = 50229,
		/// EBCDIC Japanese (Katakana) Extended.
		x_EBCDIC_JapaneseExtended = 50930,
		/// EBCDIC US-Canada and Japanese.
		x_EBCDIC_USCanadaJapanese = 50931,
		/// EBCDIC Korean Extended and Korean.
		x_EBCDIC_KoreanExtendedAndKorean = 50933,
		/// EBCDIC Simplified Chinese Extended and Simplified Chinese.
		x_EBCDIC_ChineseExtendedAndChinese = 50935,
		/// EBCDIC Simplified Chinese.
		x_EBCDIC_ChineseExtended = 50936,
		/// EBCDIC US-Canada and Traditional Chinese.
		x_EBCDIC_USCanadaChinese = 50937,
		/// EBCDIC Japanese (Latin) Extended and Japanese.
		x_EBCDIC_JapaneseExtendedAndJapanese = 50939,
		/// EUC Japanese.
		euc_jp = 51932,
		/// EUC Simplified Chinese; Chinese Simplified (EUC).
		EUC_CN = 51936,
		/// EUC Korean.
		euc_kr = 51949,
		/// EUC Traditional Chinese.
		EUC_TCN = 51950,
		/// HZ-GB2312 Simplified Chinese; Chinese Simplified (HZ).
		hz_gb_2312 = 52936,
		/// **Windows XP and later:** GB18030 Simplified Chinese (4 byte); Chinese Simplified (GB18030).
		GB18030 = 54936,
		/// ISCII Devanagari.
		x_iscii_de = 57002,
		/// ISCII Bangla.
		x_iscii_be = 57003,
		/// ISCII Tamil.
		x_iscii_ta = 57004,
		/// ISCII Telugu.
		x_iscii_te = 57005,
		/// ISCII Assamese.
		x_iscii_as = 57006,
		/// ISCII Odia.
		x_iscii_or = 57007,
		/// ISCII Kannada.
		x_iscii_ka = 57008,
		/// ISCII Malayalam.
		x_iscii_ma = 57009,
		/// ISCII Gujarati.
		x_iscii_gu = 57010,
		/// ISCII Punjabi.
		x_iscii_pa = 57011,
		/// Unicode (UTF-7).
		utf_7 = 65000,
		/// Unicode (UTF-8).
		utf_8 = 65001,
		/// Invalid code page.
		Invalid = 65535,
	}
}
impl CodePage {
	/**
	 Returns a CodePageInfo object which contains information about the CodePage.

	 # Examples
	 ```
	 # extern crate winconsole;
	 # use winconsole::console::CodePage;
	 # fn main() {
	 let info = CodePage::utf_8.get_info().unwrap();
	 println!("{}", info.name);
	 # }
	 ```

	 # Errors
 	 * [`FromUtf8Error`]: Returned if an error occurs while converting to a character.
 	 * [`FromUtf16Error`]: Returned if an error occurs while converting to a character.
 	 * [`IoError`]: Returned if an OS error occurs.

 	 [`FromUtf8Error`]: ../errors/enum.WinError.html#FromUtf8.v
 	 [`FromUtf16Error`]: ../errors/enum.WinError.html#FromUtf16.v
 	 [`IoError`]: ../errors/enum.WinError.html#Io.v
	 */
	pub fn get_info(&self) -> WinResult<CodePageInfo> {
		console::get_code_page_info(*self)
	}
}
