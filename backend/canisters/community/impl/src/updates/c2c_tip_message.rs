use crate::activity_notifications::handle_activity_notification;
use crate::{mutate_state, run_regular_jobs, RuntimeState};
use canister_api_macros::update_msgpack;
use canister_tracing_macros::trace;
use chat_events::Reader;
use community_canister::c2c_tip_message::{Response::*, *};
use group_chat_core::TipMessageResult;
use ledger_utils::format_crypto_amount_with_symbol;
use types::{ChannelMessageTipped, EventIndex, Notification};

#[update_msgpack]
#[trace]
fn c2c_tip_message(args: Args) -> Response {
    run_regular_jobs();

    mutate_state(|state| c2c_tip_message_impl(args, state))
}

fn c2c_tip_message_impl(args: Args, state: &mut RuntimeState) -> Response {
    if state.data.is_frozen() {
        return CommunityFrozen;
    }

    let user_id = state.env.caller().into();
    if let Some(member) = state.data.members.get_by_user_id(&user_id) {
        if member.suspended.value {
            return UserSuspended;
        }

        if let Some(channel) = state.data.channels.get_mut(&args.channel_id) {
            let now = state.env.now();
            let token = args.transfer.token();
            let amount = args.transfer.units();

            match channel.chat.tip_message(
                user_id,
                args.recipient,
                args.thread_root_message_index,
                args.message_id,
                args.transfer,
                now,
            ) {
                TipMessageResult::Success => {
                    if let Some((message_index, message_event_index)) = channel
                        .chat
                        .events
                        .events_reader(EventIndex::default(), args.thread_root_message_index, now)
                        .and_then(|r| {
                            r.message_event_internal(args.message_id.into())
                                .map(|e| (e.event.message_index, e.index))
                        })
                    {
                        let notification = Notification::ChannelMessageTipped(ChannelMessageTipped {
                            community_id: state.env.canister_id().into(),
                            channel_id: channel.id,
                            thread_root_message_index: args.thread_root_message_index,
                            message_index,
                            message_event_index,
                            community_name: state.data.name.clone(),
                            channel_name: channel.chat.name.clone(),
                            tipped_by: user_id,
                            tipped_by_name: args.username,
                            tipped_by_display_name: args.display_name,
                            tip: format_crypto_amount_with_symbol(amount, token.decimals().unwrap_or(8), token.token_symbol()),
                            community_avatar_id: state.data.avatar.as_ref().map(|a| a.id),
                            channel_avatar_id: channel.chat.avatar.as_ref().map(|a| a.id),
                        });
                        state.push_notification(vec![args.recipient], notification);
                    }
                    handle_activity_notification(state);
                    Success
                }
                TipMessageResult::MessageNotFound => MessageNotFound,
                TipMessageResult::CannotTipSelf => CannotTipSelf,
                TipMessageResult::RecipientMismatch => RecipientMismatch,
                TipMessageResult::UserNotInGroup => ChannelNotFound,
                TipMessageResult::NotAuthorized => NotAuthorized,
                TipMessageResult::UserSuspended => UserSuspended,
            }
        } else {
            ChannelNotFound
        }
    } else {
        UserNotInCommunity
    }
}
