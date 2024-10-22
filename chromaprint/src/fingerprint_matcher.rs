use std::cmp::Reverse;
use std::fmt::{Display, Formatter};

use crate::fingerprinter::Configuration;
use crate::gaussian::gaussian_filter;
use crate::gradient::gradient;

#[derive(Debug)]
pub enum MatchError {
    FingerprintTooLong { index: u8 },
}

impl Display for MatchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchError::FingerprintTooLong { index } => write!(f, "Fingerprint #{index} is too long"),
        }
    }
}

impl std::error::Error for MatchError {}

const ALIGN_BITS: u32 = 12;
const HASH_SHIFT: u32 = 32 - ALIGN_BITS;
const HASH_MASK: u32 = ((1 << ALIGN_BITS) - 1) << HASH_SHIFT;
const OFFSET_MASK: u32 = (1 << (32 - ALIGN_BITS - 1)) - 1;
const SOURCE_MASK: u32 = 1 << (32 - ALIGN_BITS - 1);

fn align_strip(x: u32) -> u32 {
    x >> (32 - ALIGN_BITS)
}

/// Returns similar segments of two audio streams using their fingerprints.
pub fn match_fingerprints(fp1: &[u32], fp2: &[u32], _config: &Configuration) -> Result<Vec<Segment>, MatchError> {
    if fp1.len() + 1 >= OFFSET_MASK as usize {
        return Err(MatchError::FingerprintTooLong { index: 0 });
    }

    if fp2.len() + 1 >= OFFSET_MASK as usize {
        return Err(MatchError::FingerprintTooLong { index: 1 });
    }

    let mut offsets = Vec::with_capacity(fp1.len() + fp2.len());
    for i in 0..fp1.len() {
        offsets.push((align_strip(fp1[i]) << HASH_SHIFT) | (i as u32));
    }

    for i in 0..fp2.len() {
        offsets.push((align_strip(fp2[i]) << HASH_SHIFT) | (i as u32) | SOURCE_MASK);
    }
    offsets.sort_unstable();

    let mut histogram = vec![0u32; fp1.len() + fp2.len()];
    for offset_idx in 0..offsets.len() {
        let hash1 = offsets[offset_idx] & HASH_MASK;
        let offset1 = offsets[offset_idx] & OFFSET_MASK;
        let source1 = offsets[offset_idx] & SOURCE_MASK;
        if source1 != 0 {
            // if we got hash from fp2, it means there is no hash from fp1,
            // because if there was, it would be first
            continue;
        }

        for offset_idx2 in offset_idx + 1..offsets.len() {
            let hash2 = offsets[offset_idx2] & HASH_MASK;
            if hash1 != hash2 {
                break;
            }

            let offset2 = offsets[offset_idx2] & OFFSET_MASK;
            let source2 = offsets[offset_idx2] & SOURCE_MASK;
            if source2 != 0 {
                let offset_diff = offset1 as usize + fp2.len() - offset2 as usize;
                histogram[offset_diff] += 1;
            }
        }
    }

    let mut best_alignments = Vec::new();
    let histogram_size = histogram.len();
    for i in 0..histogram_size {
        let count = histogram[i];
        if histogram[i] > 1 {
            let is_peak_left = if i > 0 { histogram[i - 1] <= count } else { true };
            let is_peak_right = if i < histogram_size - 1 { histogram[i + 1] <= count } else { true };
            if is_peak_left && is_peak_right {
                best_alignments.push((count, i));
            }
        }
    }

    best_alignments.sort_unstable_by_key(|it| Reverse(*it));

    let mut segments: Vec<Segment> = Vec::new();
    for (_count, offset) in best_alignments {
        let offset_diff = offset as isize - fp2.len() as isize;
        let offset1 = if offset_diff > 0 { offset_diff as usize } else { 0 };
        let offset2 = if offset_diff < 0 { -offset_diff as usize } else { 0 };

        let size = usize::min(fp1.len() - offset1, fp2.len() - offset2);
        let mut bit_counts = Vec::new();
        for i in 0..size {
            bit_counts.push((fp1[offset1 + i] ^ fp2[offset2 + i]).count_ones() as f64);
        }

        let orig_bit_counts = bit_counts.clone();
        let mut smoothed_bit_counts = vec![0.0; size];
        gaussian_filter(&mut bit_counts, &mut smoothed_bit_counts, 8.0, 3);

        let mut grad = Vec::with_capacity(size);
        gradient(smoothed_bit_counts.iter().copied(), &mut grad);

        for i in 0..size {
            grad[i] = grad[i].abs();
        }

        let mut gradient_peaks = Vec::new();
        for i in 0..size {
            let gi = grad[i];
            if i > 0 && i < size - 1 && gi > 0.15 && gi >= grad[i - 1] && gi >= grad[i + 1] && (gradient_peaks.is_empty() || gradient_peaks.last().unwrap() + 1 < i) {
                gradient_peaks.push(i);
            }
        }
        gradient_peaks.push(size);


        let match_threshold = 10.0;
        let max_score_difference = 0.7;

        let mut begin = 0;
        for end in gradient_peaks {
            let duration = end - begin;
            let score: f64 = orig_bit_counts[begin..end].iter().sum::<f64>() / (duration as f64);
            if score < match_threshold {
                let new_segment = Segment {
                    offset1: offset1 + begin,
                    offset2: offset2 + begin,
                    items_count: duration,
                    score,
                };

                let mut added = false;
                if let Some(s1) = segments.last_mut() {
                    if (s1.score - score).abs() < max_score_difference {
                        if let Some(merged) = s1.try_merge(&new_segment) {
                            *s1 = merged;
                            added = true;
                        }
                    }
                }

                if !added {
                    segments.push(new_segment);
                }
            }
            begin = end;
        }
        break;
    }

    Ok(segments)
}

/// Segment of an audio that is similar between two fingerprints.
#[derive(Debug)]
pub struct Segment {
    /// Index of the item in the first fingerprint.
    pub offset1: usize,

    /// Index of an item in the second fingerprint.
    pub offset2: usize,

    /// Number of items from the fingerprint corresponding to this segment.
    pub items_count: usize,

    /// Score that corresponds to similarity of this segment.
    /// The smaller this value is, the stronger similarity.
    ///
    /// This value can be be 0 up to 32.
    pub score: f64,
}

impl Segment {
    /// A timestamp representing the start of the segment in the first fingerprint.
    pub fn start1(&self, config: &Configuration) -> f32 {
        config.item_duration_in_seconds() * self.offset1 as f32
    }

    /// A timestamp representing the end of the segment in the first fingerprint.
    pub fn end1(&self, config: &Configuration) -> f32 {
        self.start1(config) + self.duration(config)
    }

    /// A timestamp representing the start of the segment in the second fingerprint.
    pub fn start2(&self, config: &Configuration) -> f32 {
        config.item_duration_in_seconds() * self.offset2 as f32
    }

    /// A timestamp representing the end of the segment in the second fingerprint.
    pub fn end2(&self, config: &Configuration) -> f32 {
        self.start2(config) + self.duration(config)
    }

    /// Duration of the segment (in seconds).
    pub fn duration(&self, config: &Configuration) -> f32 {
        config.item_duration_in_seconds() * self.items_count as f32
    }
}

impl Segment {
    /// Try to merge two consecutive segments into one.
    fn try_merge(&self, other: &Self) -> Option<Self> {
        // Check if segments are consecutive
        if self.offset1 + self.items_count != other.offset1 {
            return None;
        }

        if self.offset2 + self.items_count != other.offset2 {
            return None;
        }

        let new_duration = self.items_count + other.items_count;
        let new_score = (self.score * self.items_count as f64 + other.score * other.items_count as f64) / new_duration as f64;
        return Some(Segment {
            offset1: self.offset1,
            offset2: self.offset2,
            items_count: new_duration,
            score: new_score,
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::assert_eq_float;
    use crate::fingerprint_matcher::match_fingerprints;
    use crate::fingerprinter::Configuration;

    #[test]
    fn simple() {
        let fp1_data: [i32; 221] = [1889975932, -257508804, -240734660, -236548548, -1275161041, -1283549650, -1288796626, -1288845782, -231944646, -227770598, -227596006, -428984294, 1717901370, 1847860266, 1848063034, 1849045002, 978722826, 994451466, 1002856472, 969236792, 969204600, 2026173048, 2026215160, 2026348248, 2022681304, -393368872, -393430280, -393429272, -1462980951, -1471637847, -1475836247, -1475752279, -1408508261, -1404080485, -1404098886, -1395845206, -1260649558, -185335378, -194965314, -262004554, -262000394, -261962634, -253506554, -235820026, -235689979, -210588412, -212693436, -216889788, -216889804, -1288588764, -1288375764, -1285242323, -1309883603, -1309878993, -1578117826, -1276185314, -1284510370, -1284669458, -1284683793, -1284683779, -1284548771, -1285074676, -1284157156, -1217105619, -1242275539, -1246273217, -1246338786, -1112055010, -1118472386, -1117960918, -1088601046, -1084668806, -1101269797, -19311413, -19377013, -27759477, -15127142, -48616006, -65392726, -597817414, -589954150, -657063522, -661311010, -661643170, -644866018, -879632354, -610950114, -610948034, -606815169, -1688355556, -1184907508, -1176519156, -1109817764, -1386707332, -1390831876, -1491496216, -1491500312, -1506163992, -1557658904, -1590950929, -1607728786, -1603534722, -521333698, -525386690, -256956306, -257017634, -257017650, 1370385550, 1403976846, 1391445390, 1458405790, 1449972222, 1584197934, 1585410350, 1581195839, 1602167353, 1568612921, 1551839800, 1558123048, 1490825577, 1486627321, 1485595851, 1485616331, 416066651, 416132123, 416197643, 467577871, 442403855, 440304655, 507478045, 507150399, 507166847, 505077871, 505110766, 1046175982, 1046110463, 1062863055, 1014628813, 1014694876, 477704956, 349775612, 886646376, 886650488, 349707865, 278404699, 282599130, 299636714, 301741482, 284964010, 276582554, 276578442, 276611210, 352309434, 352236794, 352236766, 1442751822, 1476306255, 1191163167, 1191031101, 1459467068, 1585427260, 1581228348, 1579147564, 1579017580, 1579017452, 1582298364, 1607389405, 1574617551, 1557840350, 1560003070, 1551618558, 1551463550, 1547527286, 1547527414, 1547003350, 1549231830, 1557394166, -594291978, -578776362, -578767242, -679434394, -1748979865, -1211650084, -1244225828, -1319663940, -1319270740, -1287814484, -1287798084, -1287793988, -1287851028, -1287784979, -1284680593, -1586531201, -1578114017, -1594890977, -1578186993, -1578187249, -1586510305, -1553074377, -1553144522, -1553407978, -1557344201, -1590738889, -1523711658, -444714402, -436329922, -436395474, -402902226, -402910946, -419680246, -402820069, -402795479, -436208615, -453055479, -461374199, 1685898841, 1681703659, 1681654523, 1681855067, -193064325, -184667797];
        let fp2_data: [i32; 221] = [-1288792466, -1288780246, -1288845782, -231977158, -226662118, -496031718, 1718564890, 1713642554, 1847931962, 1847996442, 1047928842, 978722826, 1002840075, 969367608, 969204024, 2026173048, 2026149496, 2026348280, 2022686424, -124802344, -393368872, -393429256, -1467171160, -1471365463, -1475836247, -1475836247, -1475752263, -1404080485, -1404080485, -1395841350, -1529080918, -1261174358, -193723970, -194826082, -262004490, -261992330, -253574058, -252457978, -235821049, -235755515, -210583804, -212693436, -216889788, -216820172, -1288658396, -1284181460, -1310407891, -1309884113, -1309813458, -1309682402, -1284506338, -1284510338, -1284677649, -1284683793, -1284552739, -1284549811, -1284026100, -1284222659, -1242271443, -1246404289, -1246273250, -1246338786, -1107860706, -1117956290, -1117960918, -1084668822, -1101409030, -19303205, -19377013, -27757429, -27775861, -14996070, -48615494, -60932182, -597817414, -657063010, -661188130, -661573154, -644865954, -644784098, -611196898, -610950122, -606755777, -606815171, -614615780, -1184909812, -1109420532, -1126592932, -1403478276, -1390823684, -1491495252, -1506032984, -1506065748, -1574436177, -1591213074, -1607729106, -529792978, -521391058, -257087442, -256956290, 1890465998, 1940813966, 1387191438, 1391363470, 1382908302, 1449955518, 1450045678, 1580068910, 1580146991, 1596858925, 1567564345, 1551839801, 1560220201, 1560089131, 1486627627, 1486628186, 1485628490, 1485614154, 411872346, 412003406, 432978958, 467573775, 442401805, 440370205, 507478068, 507166772, 440057957, 437936357, 442196199, 442196199, 450506951, 536477893, 477823445, 477696756, 486089340, 483993196, 483993196, 479805048, 345521737, 345517771, 362299386, 299373994, 299636138, 274464954, 8126602, 8142986, 16568474, 16707770, 12498170, 79602718, 1157539086, 1174381855, 1207805245, 1191031068, 1191153948, 1182699804, 1444852028, 1579066732, 1579148780, 1579017452, 1583342780, 1600050589, 534430095, 534496159, 1568391646, 1568232830, 1568502830, 1547527214, 1547002918, 1551327574, 1551262294, 1553453654, -577387946, -578776490, -545216905, -1752912011, -1748652076, -1782071596, -1780511052, -1327659348, -1319271764, -1317240148, -1287863636, -1287851347, -1288834067, -1288834577, -1322396185, -1603406729, -1594889129, -1594950377, -1594962681, -1594962169, -1586505929, -1553078985, -1574313705, -1574309881, -1574223817, -1557462681, -1486220425, -436329866, -436395410, -440651217, -436456674, -402902770, -402836469, -402820037, -402817991, -436208631, -436225015, -167993525, 1971094107, 1966916347, 1966953050, 1967002202, -176287382, -184668054, -188926869, -188992263, -188777271, -188818231, -33628727];

        let fp1 = fp1_data.iter().copied().map(|c| c as u32).collect::<Vec<_>>();
        let fp2 = fp2_data.iter().copied().map(|c| c as u32).collect::<Vec<_>>();

        let conf = Configuration::preset_test2();
        let segments = match_fingerprints(&fp1, &fp2, &conf).unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].offset1, 5);
        assert_eq!(segments[0].offset2, 0);
        assert_eq!(segments[0].items_count, 216);
        assert_eq_float!(segments[0].score, 3.17183, 0.001);
    }
}
