// Copyright 2019 The xi-editor Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Simple countdown timer

use druid::widget::{Button, Flex, Label, MainAxisAlignment, Painter};
use druid::{
    theme, AppLauncher, BoxConstraints, Color, Data, Env, Event, EventCtx, LayoutCtx, Lens,
    LifeCycle, LifeCycleCtx, LocalizedString, PaintCtx, PlatformError, RenderContext, Selector,
    Size, Target, TimerToken, UpdateCtx, Widget, WidgetExt, WidgetId, WindowDesc,
};
use std::time::{Duration, Instant};

const TIMER_UPDATE_DELAY: Duration = Duration::from_millis(50);

const ROOT_WIDGET_ID: WidgetId = WidgetId::reserved(1);
const CMD_START_TIMER: Selector = Selector::new("start_paint_timer");
const CMD_STOP_TIMER: Selector = Selector::new("stop_paint_timer");

#[derive(Clone, Debug, PartialEq)]
enum TimerState {
    Init,
    Running {
        started_at: Instant,
        duration: Duration,
    },
    Stopped {
        duration: Duration,
    },
    Completed,
}

#[derive(Clone, Lens, Data)]
struct AppData {
    text: String,
    #[druid(same_fn = "PartialEq::eq")]
    duration: Duration,
    #[druid(same_fn = "PartialEq::eq")]
    timer_state: TimerState,
}

impl AppData {
    fn update(&mut self) {
        self.text = match self.timer_state {
            TimerState::Init => duration_as_human_readable(self.duration),
            TimerState::Running {
                started_at,
                duration,
            } => {
                let duration_passed = Instant::now() - started_at;
                let leftover_duration = duration.checked_sub(duration_passed);
                if let Some(leftover_duration) = leftover_duration {
                    duration_as_human_readable(leftover_duration)
                } else {
                    self.timer_state = TimerState::Completed;
                    return self.update();
                }
            }
            TimerState::Stopped { duration } => duration_as_human_readable(duration),
            TimerState::Completed => duration_as_human_readable(Duration::from_secs(0)),
        };
    }
}

struct RootWidget<T: Widget<AppData>> {
    timer_id: TimerToken,
    inner: T,
}

impl<T: Widget<AppData>> RootWidget<T> {
    fn new(inner: T) -> Self {
        Self {
            timer_id: TimerToken::INVALID,
            inner,
        }
    }
}

fn main() -> Result<(), PlatformError> {
    let main_window = WindowDesc::new(ui_builder).title(
        LocalizedString::new("styled-text-demo-window-title").with_placeholder("Type Styler"),
    );

    let default_duration = Duration::from_secs(5 * 60);
    let data = AppData {
        text: duration_as_human_readable(default_duration),
        duration: default_duration,
        timer_state: TimerState::Init,
    };

    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)?;

    Ok(())
}

impl<T: Widget<AppData>> Widget<AppData> for RootWidget<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut AppData, env: &Env) {
        let is_handled = match event {
            Event::Timer(id) => {
                if *id == self.timer_id {
                    let deadline = Instant::now() + TIMER_UPDATE_DELAY;
                    self.timer_id = ctx.request_timer(deadline);
                    data.update();
                    true
                } else {
                    false
                }
            }
            Event::Command(cmd) => {
                if cmd.selector == CMD_START_TIMER {
                    let deadline = Instant::now() + TIMER_UPDATE_DELAY;
                    self.timer_id = ctx.request_timer(deadline);
                    data.update();
                    true
                } else if cmd.selector == CMD_STOP_TIMER {
                    self.timer_id = TimerToken::INVALID;
                    data.update();
                    true
                } else {
                    false
                }
            }
            _ => false,
        };

        if !is_handled {
            self.inner.event(ctx, event, data, env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &AppData, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &AppData, data: &AppData, env: &Env) {
        self.inner.update(ctx, old_data, data, env);
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &AppData,
        env: &Env,
    ) -> Size {
        self.inner.layout(layout_ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AppData, env: &Env) {
        self.inner.paint(ctx, data, env);
    }

    fn id(&self) -> Option<WidgetId> {
        Some(ROOT_WIDGET_ID)
    }
}

fn ui_builder() -> impl Widget<AppData> {
    let my_painter = Painter::new(|ctx, _, _| {
        let bounds = ctx.size().to_rect();
        if ctx.is_hot() {
            ctx.fill(bounds, &Color::rgba8(0, 0, 0, 128));
        }

        if ctx.is_active() {
            ctx.stroke(bounds, &Color::WHITE, 2.0);
        }
    });

    let styled_label = Label::new(|data: &AppData, _env: &_| data.text.clone())
        .with_text_color(theme::PRIMARY_LIGHT)
        .with_text_size(24.0)
        .background(my_painter);

    let start_button = Button::new("Start")
        .on_click(|ctx: &mut EventCtx, data: &mut AppData, _| {
            match data.timer_state {
                TimerState::Init => {
                    data.timer_state = TimerState::Running {
                        started_at: Instant::now(),
                        duration: data.duration,
                    };
                }
                TimerState::Stopped { duration } => {
                    data.timer_state = TimerState::Running {
                        started_at: Instant::now(),
                        duration,
                    };
                }
                _ => (),
            }

            // Targetting ROOT_WIDGET_ID doesn't works
            ctx.submit_command(CMD_START_TIMER, ROOT_WIDGET_ID);
        })
        .fix_height(30.0);

    let stop_button = Button::new("Stop")
        .on_click(|ctx: &mut EventCtx, data: &mut AppData, _| {
            if let TimerState::Running {
                started_at,
                duration,
            } = data.timer_state
            {
                let duration_passed = Instant::now() - started_at;
                let leftover_duration = duration.checked_sub(duration_passed);
                if let Some(leftover_duration) = leftover_duration {
                    data.timer_state = TimerState::Stopped {
                        duration: leftover_duration,
                    };
                } else {
                    data.timer_state = TimerState::Completed;
                }
            }
            // Targetting ROOT_WIDGET_ID doesn't work
            ctx.submit_command(CMD_STOP_TIMER, ROOT_WIDGET_ID);
        })
        .fix_height(30.0);

    let reset_button = Button::new("Reset")
        .on_click(|ctx: &mut EventCtx, data: &mut AppData, _| {
            data.timer_state = TimerState::Init;
            // Targetting ROOT_WIDGET_ID doesn't work
            ctx.submit_command(CMD_STOP_TIMER, ROOT_WIDGET_ID);
        })
        .fix_height(30.0);

    let layout_child = Flex::column()
        .main_axis_alignment(MainAxisAlignment::Center)
        .with_child(styled_label)
        .with_spacer(8.0)
        .with_child(
            Flex::row()
                .with_child(start_button)
                .with_spacer(5.0)
                .with_child(stop_button)
                .with_spacer(5.0)
                .with_child(reset_button),
        );

    RootWidget::new(layout_child)
}

fn duration_as_human_readable(duration: Duration) -> String {
    let secs_total = duration.as_secs();
    let secs = secs_total % 60;

    let mins_total = secs_total / 60;
    let mins = mins_total % 60;

    // For simplicity sake we do not go further into days/years etc.
    let hours = mins_total / 60;

    format!("{:02}:{:02}:{:02}", hours, mins, secs)
}
