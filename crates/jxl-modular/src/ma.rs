use std::{io::Read, collections::VecDeque};
use std::sync::Arc;

use jxl_bitstream::{unpack_signed, Bitstream, Bundle};
use jxl_coding::Decoder;

use crate::Result;
use super::predictor::{Predictor, Properties};

#[derive(Debug, Clone)]
pub struct MaConfig {
    tree: Arc<MaTreeNode>,
    decoder: Decoder,
}

impl MaConfig {
    pub fn decoder(&self) -> &Decoder {
        &self.decoder
    }

    pub fn make_flat_tree(&self, channel: u32, stream_idx: u32) -> FlatMaTree {
        let mut nodes = Vec::new();
        self.tree.flatten(channel, stream_idx, &mut nodes);
        FlatMaTree::new(nodes)
    }
}

impl<Ctx> Bundle<Ctx> for MaConfig {
    type Error = crate::Error;

    fn parse<R: Read>(bitstream: &mut Bitstream<R>, _: Ctx) -> crate::Result<Self> {
        enum FoldingTree {
            Decision(u32, i32),
            Leaf(MaTreeLeaf),
        }

        let mut tree_decoder = Decoder::parse(bitstream, 6)?;
        let mut ctx = 0u32;
        let mut nodes_left = 1usize;
        let mut nodes = Vec::new();

        tree_decoder.begin(bitstream)?;
        while nodes_left > 0 {
            if nodes.len() >= (1 << 26) {
                return Err(crate::Error::InvalidMaTree);
            }

            nodes_left -= 1;
            let property = tree_decoder.read_varint(bitstream, 1)?;
            let node = if let Some(property) = property.checked_sub(1) {
                let value = unpack_signed(tree_decoder.read_varint(bitstream, 0)?);
                let node = FoldingTree::Decision(property, value);
                nodes_left += 2;
                node
            } else {
                let predictor = tree_decoder.read_varint(bitstream, 2)?;
                let predictor = Predictor::try_from(predictor)?;
                let offset = unpack_signed(tree_decoder.read_varint(bitstream, 3)?);
                let mul_log = tree_decoder.read_varint(bitstream, 4)?;
                if mul_log > 30 {
                    return Err(crate::Error::InvalidMaTree);
                }
                let mul_bits = tree_decoder.read_varint(bitstream, 5)?;
                if mul_bits > (1 << (31 - mul_log)) - 2 {
                    return Err(crate::Error::InvalidMaTree);
                }
                let multiplier = (mul_bits + 1) << mul_log;
                let node = FoldingTree::Leaf(MaTreeLeaf {
                    ctx,
                    predictor,
                    offset,
                    multiplier,
                });
                ctx += 1;
                node
            };
            nodes.push(node);
        }
        tree_decoder.finalize()?;

        let mut tmp = VecDeque::new();
        for node in nodes.into_iter().rev() {
            match node {
                FoldingTree::Decision(property, value) => {
                    let right = tmp.pop_front().unwrap();
                    let left = tmp.pop_front().unwrap();
                    tmp.push_back(MaTreeNode::Decision {
                        property,
                        value,
                        left: Box::new(left),
                        right: Box::new(right),
                    });
                },
                FoldingTree::Leaf(leaf) => {
                    tmp.push_back(MaTreeNode::Leaf(leaf));
                },
            }
        }
        assert_eq!(tmp.len(), 1);
        let tree = tmp.pop_front().unwrap();

        let decoder = Decoder::parse(bitstream, ctx)?;
        Ok(Self {
            tree: Arc::new(tree),
            decoder,
        })
    }
}

#[derive(Debug)]
pub struct FlatMaTree {
    nodes: Vec<FlatMaTreeNode>,
    need_self_correcting: bool,
}

#[derive(Debug)]
enum FlatMaTreeNode {
    Decision {
        property: u32,
        value: i32,
        left_idx: u32,
        right_idx: u32,
    },
    Leaf(MaTreeLeaf),
}

#[derive(Debug, Clone)]
struct MaTreeLeaf {
    ctx: u32,
    predictor: super::predictor::Predictor,
    offset: i32,
    multiplier: u32,
}

impl FlatMaTree {
    fn new(nodes: Vec<FlatMaTreeNode>) -> Self {
        let need_self_correcting = nodes.iter().any(|node| match *node {
            FlatMaTreeNode::Decision { property, .. } => property == 15,
            FlatMaTreeNode::Leaf(MaTreeLeaf { predictor, .. }) => predictor == Predictor::SelfCorrecting,
        });

        Self { nodes, need_self_correcting }
    }

    fn get_leaf(&self, properties: &Properties) -> Result<&MaTreeLeaf> {
        let mut current_node = &self.nodes[0];
        loop {
            match current_node {
                &FlatMaTreeNode::Decision { property, value, left_idx, right_idx } => {
                    let prop_value = properties.get(property as usize)?;
                    let next_node = if prop_value > value { left_idx } else { right_idx };
                    current_node = &self.nodes[next_node as usize];
                },
                FlatMaTreeNode::Leaf(leaf) => return Ok(leaf),
            }
        }
    }
}

impl FlatMaTree {
    pub fn need_self_correcting(&self) -> bool {
        self.need_self_correcting
    }

    pub fn decode_sample<R: Read>(
        &self,
        bitstream: &mut Bitstream<R>,
        decoder: &mut Decoder,
        properties: &Properties,
        dist_multiplier: u32,
    ) -> Result<(i32, super::predictor::Predictor)> {
        let leaf = self.get_leaf(properties)?;
        let diff = decoder.read_varint_with_multiplier(bitstream, leaf.ctx, dist_multiplier)?;
        let diff = unpack_signed(diff) * leaf.multiplier as i32 + leaf.offset;
        Ok((diff, leaf.predictor))
    }
}

#[derive(Debug)]
enum MaTreeNode {
    Decision {
        property: u32,
        value: i32,
        left: Box<MaTreeNode>,
        right: Box<MaTreeNode>,
    },
    Leaf(MaTreeLeaf),
}

impl MaTreeNode {
    fn flatten(&self, channel: u32, stream_idx: u32, out: &mut Vec<FlatMaTreeNode>) {
        let idx = out.len();
        match *self {
            MaTreeNode::Decision { property, value, ref left, ref right } => {
                if property == 0 || property == 1 {
                    let target = if property == 0 { channel } else { stream_idx };
                    let branch = if target as i32 > value { left } else { right };
                    return branch.flatten(channel, stream_idx, out);
                }

                out.push(FlatMaTreeNode::Decision {
                    property,
                    value,
                    left_idx: 0,
                    right_idx: 0,
                });
                left.flatten(channel, stream_idx, out);
                let right_idx = out.len() as u32;
                right.flatten(channel, stream_idx, out);
                out[idx] = FlatMaTreeNode::Decision {
                    property,
                    value,
                    left_idx: (idx + 1) as u32,
                    right_idx,
                };
            },
            MaTreeNode::Leaf(ref leaf) => {
                out.push(FlatMaTreeNode::Leaf(leaf.clone()));
            },
        }
    }
}
