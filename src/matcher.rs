use std::sync::mpsc::{Receiver, Sender};
use event::{Event, EventArg};
use item::{Item, MatchedItem, MatchedRange};
use std::sync::Arc;

use std::io::Write;
use score;

macro_rules! println_stderr(
    ($($arg:tt)*) => { {
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
    } }
);

#[derive(Clone, Copy)]
enum Algorithm {
    FUZZY,
    REGEX,
}

pub struct Matcher {
    tx_result: Sender<(Event, EventArg)>,
    rx_item: Receiver<(Event, EventArg)>,
    rank_criterion: Arc<Vec<RankCriteria>>,
}

impl Matcher {
    pub fn new(rx_item: Receiver<(Event, EventArg)>, tx_result: Sender<(Event, EventArg)>) -> Self {
        Matcher {
            rx_item: rx_item,
            tx_result: tx_result,
            rank_criterion: Arc::new(Vec::new()),
        }
    }


    pub fn run(&self) {
        let mut query = "".to_string();
        let mut matcher_engine: Option<MatchingEngine> = None;
        while let Ok((ev, arg)) = self.rx_item.recv() {
            match ev {
                Event::EvMatcherNewItem => {
                    let item = *arg.downcast::<Item>().unwrap();

                    // TODO: filter logic
                    matcher_engine.as_ref().map(|mat| {
                        let matched_item = mat.match_item(item);
                        if matched_item != None {
                            self.tx_result.send((Event::EvModelNewItem, Box::new(matched_item.unwrap())));
                        }
                    });
                }

                Event::EvReaderEnd => {
                    // reader had stopped,
                }

                Event::EvMatcherRestart => {
                    query = *arg.downcast::<String>().unwrap();

                    // notifiy the model that the query had been changed
                    self.tx_result.send((Event::EvModelRestart, Box::new(true)));

                    matcher_engine = Some(MatchingEngine::builder(&query).build());
                }

                _ => {}
            }
        }
    }

}

struct MatchingEngine<'a> {
    query: String,
    query_chars: Vec<char>,
    query_lower_chars: Vec<char>,
    rank_criterion: Option<&'a [RankCriteria]>,
    weights: Option<i32>, // not used
}

impl<'a> MatchingEngine<'a> {
    pub fn builder(query: &str) -> Self {
        MatchingEngine {
            query: query.to_string(),
            query_chars: query.chars().collect(),
            query_lower_chars: query.to_lowercase().chars().collect(),
            rank_criterion: None,
            weights: None,
        }
    }

    pub fn rank(mut self, rank: &'a [RankCriteria]) -> Self {
        self.rank_criterion = Some(rank);
        self
    }

    pub fn build(self) -> Self {
        self
    }

    pub fn match_item(&self, item: Item) -> Option<MatchedItem> {
        //match algorithm {
            //Algorithm::FUZZY => self.match_item_fuzzy(item),
            //Algorithm::REGEX => self.match_item_regex(item),
        //}
        self.match_item_fuzzy(item)
    }


    fn build_rank(&self, score: i64, index: i64, begin: i64, end: i64) -> [i64; 4] {
        self.rank_criterion.map(|criterion| {
            let mut rank = [0; 4];
            for (idx, criteria) in criterion.iter().enumerate().take(4) {
                rank[idx] = match *criteria {
                    RankCriteria::Score    => score,
                    RankCriteria::Index    => index,
                    RankCriteria::Begin    => begin,
                    RankCriteria::End      => end,
                    RankCriteria::NegScore => -score,
                    RankCriteria::NegIndex => -index,
                    RankCriteria::NegBegin => -begin,
                    RankCriteria::NegEnd   => -end,
                }
            }
            rank
        }).unwrap_or([0; 4])
    }

    //fn match_item_regex(&self, item: &Item) -> Option<MatchedItem> {
        //let matched_result = if self.query.empty() {
            //Some((0, 0))
        //} else {
            //score::regex_match(item.get_text(), self.query.get_regex())
        //};

        //if matched_result == None {
            //return None;
        //}

        //let (begin, end) = matched_result.unwrap();

        //if !self.query.empty() && !item.in_matching_range(begin, end) {
            //return None;
        //}

        //let score = end - begin;
        //let rank = self.build_rank(score as i64, index as i64, begin as i64, end as i64);

        //let mut item = MatchedItem::new(index);
        //item.set_matched_range(MatchedRange::Range(begin, end));
        //item.set_rank(rank);
        //Some(item)
    //}

    fn match_item_fuzzy(&self, item: Item) -> Option<MatchedItem> {
        // iterate over all matching fields:
        let mut matched_result = None;
        for &(start, end) in item.get_matching_ranges() {
            let source = &item.get_lower_chars()[start .. end];

            matched_result = score::fuzzy_match(source, &self.query_chars, &self.query_lower_chars);

            if matched_result == None {
                continue;
            }
        }

        if matched_result == None {
            return None;
        }

        let (score, matched_range) = matched_result.unwrap();

        let begin = *matched_range.get(0).unwrap_or(&0) as i64;
        let end = *matched_range.last().unwrap_or(&0) as i64;

        let rank = self.build_rank(-score, item.get_index() as i64, begin, end);

        Some(MatchedItem::builder(item)
             .rank(rank)
             .matched_range(MatchedRange::Chars(matched_range))
             .build())
    }
}

pub enum RankCriteria {
    Score,
    Index,
    Begin,
    End,
    NegScore,
    NegIndex,
    NegBegin,
    NegEnd,
}

pub fn parse_criteria(text: &str) -> Option<RankCriteria> {
    match text.to_lowercase().as_ref() {
        "score"  => Some(RankCriteria::Score),
        "index"  => Some(RankCriteria::Index),
        "begin"  => Some(RankCriteria::Begin),
        "end"    => Some(RankCriteria::End),
        "-score" => Some(RankCriteria::NegScore),
        "-index" => Some(RankCriteria::NegIndex),
        "-begin" => Some(RankCriteria::NegBegin),
        "-end"   => Some(RankCriteria::NegEnd),
        _ => None,
    }
}
