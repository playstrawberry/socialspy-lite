use crate::{
    lang,
    model::{Spies, Toggle},
};
use sp_social::{MESSAGE_PLUGIN, SPY_NODE, runtime::id};
use std::sync::{Arc, Mutex, MutexGuard};
use strawberry_plugin_api::{
    Context, EventHandler, Plugin, PluginMetadata, Result, Server,
    command::{Command, CommandError, CommandSender, ConsumedArgs},
    commands::CommandHandler,
    common::NamedColor,
    events::{FailureVerdict, HandlerOptions, HandlerPriority, PlayerLeaveEvent},
    events_wit::PlayerLeaveEventData,
    permission::{Permission, PermissionDefault, PermissionLevel},
    register_plugin,
    text::TextComponent,
    wit,
};

type Shared = Arc<Mutex<Spies>>;

fn lock(spies: &Shared) -> MutexGuard<'_, Spies> {
    spies
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

struct Run(Shared);

impl CommandHandler for Run {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> std::result::Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            return Err(CommandError::CommandFailed(TextComponent::text(
                &lang::text("en", "players_only"),
            )));
        };
        let locale = player.get_locale();
        let refuse = |key: &str| {
            CommandError::CommandFailed(
                TextComponent::text(&lang::text(&locale, key)).color_named(NamedColor::Red),
            )
        };
        if !player.has_permission(SPY_NODE) {
            return Err(refuse("no_permission"));
        }
        let toggle = lock(&self.0).toggle(id(&player));
        let key = match toggle {
            Toggle::On => "on",
            Toggle::Off => "off",
            Toggle::Full => return Err(refuse("full")),
        };
        player.send_system_message(
            TextComponent::text(&lang::text(&locale, key)).color_named(NamedColor::Gray),
            false,
        );
        Ok(1)
    }
}

struct Events(Shared);

impl EventHandler<PlayerLeaveEvent> for Events {
    fn handle(&self, _server: Server, event: PlayerLeaveEventData) -> PlayerLeaveEventData {
        lock(&self.0).forget(id(&event.player));
        event
    }
}

struct SocialSpyLite {
    spies: Shared,
}

impl Plugin for SocialSpyLite {
    fn new() -> Self {
        Self {
            spies: Arc::new(Mutex::new(Spies::default())),
        }
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "socialspy-lite".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            authors: vec!["Strawberry Interactive LLC".into()],
            description: lang::text("en", "description"),
            dependencies: vec![],
            permissions: vec![],
        }
    }

    fn on_load(&self, context: Context) -> Result<()> {
        context.register_permission(&Permission {
            node: SPY_NODE.into(),
            description: lang::text("en", "permission_use"),
            default: PermissionDefault::Op(PermissionLevel::Two),
            children: vec![],
        })?;
        context.register_command(
            Command::new(
                &["socialspy".into()],
                &lang::text("en", "command_socialspy"),
            )
            .execute(Run(Arc::clone(&self.spies))),
            SPY_NODE,
        );
        // A leave is only watched; a hidden leave line (a cancelled event) is still a leave.
        context.register_event_handler_with_options::<PlayerLeaveEvent, _>(
            Events(Arc::clone(&self.spies)),
            HandlerOptions {
                priority: HandlerPriority::Monitor,
                blocking: true,
                ignore_cancelled: false,
                on_failure: FailureVerdict::Allow,
            },
        )?;
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<()> {
        lock(&self.spies).clear();
        Ok(())
    }

    /// `spies/1`: who switched social spy on. Answered to the private-message script only; who the
    /// spies are is nobody else's business.
    fn handle_ipc_message(
        &self,
        sender: wit::PluginId,
        message: wit::IpcMessage,
    ) -> std::result::Result<wit::IpcMessage, String> {
        if sender != MESSAGE_PLUGIN || !sp_social::is_spies_request(&message) {
            return Err("not a spies/1 question from msg-lite".into());
        }
        Ok(sp_social::spies_answer(&lock(&self.spies).list()))
    }
}

register_plugin!(SocialSpyLite);
