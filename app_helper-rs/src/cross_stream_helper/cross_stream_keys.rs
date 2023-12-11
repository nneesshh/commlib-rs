use commlib::{NodeId, SpecialZone, ZoneId};

///
#[inline(always)]
pub fn make_streams_ids_pair(
    stream_ids: &hashbrown::HashMap<String, String>,
) -> (Vec<String>, Vec<String>) {
    let mut streams = Vec::with_capacity(stream_ids.len());
    let mut ids = Vec::with_capacity(stream_ids.len());

    //
    for (stream, id) in stream_ids {
        streams.push(stream.clone());
        ids.push(id.clone());
    }

    (streams, ids)
}

///
#[inline(always)]
pub fn stream_id_for_zone(zone: ZoneId) -> String {
    std::format!("mq:zone:{}", zone)
}

///
#[inline(always)]
pub fn stream_id_for_cross_node(cross: NodeId) -> String {
    std::format!("mq:cross:{}", cross)
}

///
#[inline(always)]
pub fn stream_id_for_social_node(social: NodeId) -> String {
    std::format!("mq:social:cross:{}", social)
}

///
#[inline(always)]
pub fn stream_id_for_mechanics_node(mechanics: NodeId) -> String {
    std::format!("mq:mechanics:cross:{}", mechanics)
}

///
#[inline(always)]
pub fn stream_id_for_guild_node(guild: NodeId) -> String {
    std::format!("mq:guild:cross:{}", guild)
}

/// 世界聊天管理节点
#[inline(always)]
pub fn stream_id_for_world_chat_manager() -> String {
    "mq:worldchat:mng".to_owned()
}

/// 世界聊天频道
#[inline(always)]
pub fn stream_id_for_world_chat_channel(channel: i32) -> String {
    std::format!("mq:worldchat:ch:{}", channel)
}

///
#[inline(always)]
pub fn stream_id_for_lobby_node(lobby: NodeId) -> String {
    std::format!("mq:lobby:{}", lobby)
}

///
#[inline(always)]
pub fn get_down_stream_name(zone: ZoneId) -> String {
    assert!(zone > 0);
    stream_id_for_zone(zone)
}

///
#[inline(always)]
pub fn get_up_stream_name(sp_zone: SpecialZone, node: NodeId, channel: i32) -> String {
    assert!((sp_zone as i8) < 0_i8);
    match sp_zone {
        SpecialZone::Cross => {
            //
            stream_id_for_cross_node(node)
        }
        SpecialZone::WorldChatMng => {
            //
            stream_id_for_world_chat_manager()
        }
        SpecialZone::WorldChatChannel => {
            //
            stream_id_for_world_chat_channel(channel)
        }
        SpecialZone::Social => {
            //
            stream_id_for_social_node(node)
        }
        SpecialZone::Mechanics => {
            //
            stream_id_for_mechanics_node(node)
        }
        SpecialZone::Lobby => {
            //
            stream_id_for_lobby_node(node)
        }
    }
}
