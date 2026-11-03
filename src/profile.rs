//! 协议端画像，对齐 SnowLuma-Adapter 出站 codec 的画像分支
//!（`profile.py` 的 msg_action_style / split_file_upload / forward_param_key）。
//!
//! NapCat 画像是通用模型的特例（单发送动作、文件内联、forward 参数键 message）。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgActionStyle {
    /// NapCat：send_group_msg / send_private_msg
    Targeted,
    /// SnowLuma：通用 send_msg + message_type（group/private）
    Generic,
}

#[derive(Debug, Clone)]
pub struct ProtocolProfile {
    pub name: &'static str,
    pub msg_action_style: MsgActionStyle,
    /// true（SnowLuma）：file 段拆为独立 upload_*_file 动作，保持与文本段相对顺序；
    /// false（NapCat）：file 段内联随消息发送
    pub split_file_upload: bool,
    /// 合并转发动作的消息键："message"（NapCat）/ "messages"（SnowLuma）
    pub forward_param_key: &'static str,
}

impl ProtocolProfile {
    pub fn napcat() -> Self {
        ProtocolProfile {
            name: "napcat",
            msg_action_style: MsgActionStyle::Targeted,
            split_file_upload: false,
            forward_param_key: "message",
        }
    }

    pub fn snowluma() -> Self {
        ProtocolProfile {
            name: "snowluma",
            msg_action_style: MsgActionStyle::Generic,
            split_file_upload: true,
            forward_param_key: "messages",
        }
    }

    pub fn from_name(name: &str) -> Self {
        if name.trim().eq_ignore_ascii_case("napcat") {
            Self::napcat()
        } else {
            Self::snowluma()
        }
    }
}
