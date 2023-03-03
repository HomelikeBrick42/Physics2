use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use cgmath::prelude::*;
use eframe::egui;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    get_collision, CameraUniform, Collider, Quad, Renderer, StorageBufferQuad, SweepingCollider,
    MAX_PHYSICS_ITERATIONS,
};

#[derive(Serialize, Deserialize)]
pub struct Camera {
    position: cgmath::Vector2<f32>,
    rotation: f32,
    zoom: f32,
}

#[derive(Serialize, Deserialize)]
pub struct App {
    #[serde(skip, default = "std::time::Instant::now")]
    last_time: std::time::Instant,
    #[serde(skip)]
    fixed_update_time: std::time::Duration,
    info_window_open: bool,
    settings_window_open: bool,
    quads_window_open: bool,
    physics_enabled: bool,
    sweeping_colliders: bool,
    gravity: cgmath::Vector2<f32>,
    camera: Camera,
    quads: Vec<Quad>,
    old_quads: Vec<Quad>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            last_time: std::time::Instant::now(),
            fixed_update_time: std::time::Duration::ZERO,
            info_window_open: false,
            settings_window_open: false,
            quads_window_open: false,
            physics_enabled: false,
            sweeping_colliders: false,
            gravity: cgmath::vec2(0.0, -9.81),
            camera: Camera {
                position: cgmath::vec2(0.0, 0.0),
                rotation: 0.0,
                zoom: 0.25,
            },
            quads: vec![
                Quad {
                    position: cgmath::vec2(0.0, 0.0),
                    velocity: cgmath::vec2(0.0, 0.0),
                    rotation: 0.0,
                    angular_velocity: 0.0,
                    scale: cgmath::vec2(1.0, 1.0),
                    color: cgmath::vec3(0.1, 0.2, 0.8),
                    dynamic: true,
                },
                Quad {
                    position: cgmath::vec2(0.0, -2.0),
                    velocity: cgmath::vec2(0.0, 0.0),
                    rotation: 0.0,
                    angular_velocity: 0.0,
                    scale: cgmath::vec2(5.0, 0.5),
                    color: cgmath::vec3(0.3, 0.8, 0.2),
                    dynamic: false,
                },
            ],
            old_quads: vec![],
        }
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        {
            let render_state = cc.wgpu_render_state.as_ref().unwrap();
            let renderer = Renderer::new(
                &render_state.device,
                &render_state.queue,
                render_state.target_format,
            );
            let old_value = render_state
                .renderer
                .write()
                .paint_callback_resources
                .insert(renderer);
            assert!(old_value.is_none());
        }

        cc.storage
            .map(|s| serde_json::from_str(s.get_string("App").as_deref().unwrap_or("")))
            .transpose()
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    fn update(&mut self, _ts: f32) {}

    fn fixed_update(&mut self, ts: f32) {
        self.quads
            .par_iter_mut()
            .filter(|quad| quad.dynamic)
            .for_each(|quad| {
                quad.velocity += self.gravity * ts;
            });

        let solved = AtomicBool::new(false);
        let mut iterations = 0;
        while !solved.load(Ordering::Relaxed) && iterations < MAX_PHYSICS_ITERATIONS {
            solved.store(true, Ordering::Relaxed);

            std::mem::swap(&mut self.quads, &mut self.old_quads);
            self.quads.clear();
            self.quads.reserve(self.old_quads.len());
            self.quads
                .par_extend(
                    self.old_quads
                        .par_iter()
                        .enumerate()
                        .map(|(index, &(mut quad))| {
                            if quad.dynamic {
                                let mut position_delta = cgmath::vec2(0.0, 0.0);
                                let mut velocity_delta = cgmath::vec2(0.0, 0.0);

                                // TODO: spacial hashing so we dont have to iterate through every object in the scene
                                for (other_index, other) in self.old_quads.iter().enumerate() {
                                    if other_index != index {
                                        let sweeping_collider = SweepingCollider {
                                            collider: &quad,
                                            position_a: quad.position,
                                            position_b: (quad.position + position_delta)
                                                + (quad.velocity + velocity_delta) * ts,
                                        };

                                        let sweeping_collider_other = SweepingCollider {
                                            collider: other,
                                            position_a: other.position,
                                            position_b: other.position + other.velocity * ts,
                                        };

                                        let (collider_a, collider_b): (
                                            &dyn Collider,
                                            &dyn Collider,
                                        ) = if self.sweeping_colliders {
                                            (&sweeping_collider, &sweeping_collider_other)
                                        } else {
                                            (&quad, other)
                                        };

                                        if let Some(collision) =
                                            get_collision(collider_a, collider_b)
                                        {
                                            let relative_velocity = other.velocity - quad.velocity;
                                            let collision_normal_velocity_length =
                                                relative_velocity.dot(-collision.normal);
                                            if collision_normal_velocity_length >= 0.0 {
                                                // A collision has happened, so the physics is not solved
                                                solved.store(false, Ordering::Relaxed);

                                                let dynamic_count =
                                                    quad.dynamic as usize + other.dynamic as usize;

                                                if let Some(collision) = get_collision(&quad, other)
                                                {
                                                    // Move the quad out of collision
                                                    position_delta -= collision.normal
                                                        * collision.depth
                                                        / dynamic_count as _;
                                                }

                                                // Stop movement in that direction
                                                velocity_delta -= (-relative_velocity)
                                                    .dot(collision.normal)
                                                    * collision.normal;
                                            }
                                        }
                                    }
                                }

                                quad.position += position_delta;
                                quad.velocity += velocity_delta;
                            }
                            quad
                        }),
                );

            iterations += 1;
        }

        if iterations == MAX_PHYSICS_ITERATIONS {
            println!("Warning: reached maximum physics iterations, the simulation may be unstable");
        }

        self.quads
            .par_iter_mut()
            .filter(|quad| quad.dynamic)
            .for_each(|quad| {
                quad.position += quad.velocity * ts;
                quad.rotation += quad.angular_velocity * ts;
            });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let time = std::time::Instant::now();
        let dt = time.duration_since(self.last_time);
        self.last_time = time;

        let ts = dt.as_secs_f32();

        Self::update(self, ts);

        let fixed_update_start = std::time::Instant::now();
        if self.physics_enabled {
            self.fixed_update_time += dt;
            let fixed_update_interval = std::time::Duration::from_secs_f64(1.0 / 100.0);
            while self.fixed_update_time > fixed_update_interval {
                self.fixed_update(fixed_update_interval.as_secs_f32());
                self.fixed_update_time -= fixed_update_interval;
            }
        }
        let fixed_update_duration = fixed_update_start.elapsed();

        // Make sure rotations dont get too high
        self.quads.par_iter_mut().for_each(|quad| {
            quad.rotation %= std::f32::consts::TAU;
            quad.rotation += std::f32::consts::TAU;
            quad.rotation %= std::f32::consts::TAU;
        });

        egui::TopBottomPanel::top("Top Panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.info_window_open |= ui.button("Info").clicked();
                self.settings_window_open |= ui.button("Settings").clicked();
                self.quads_window_open |= ui.button("Quads").clicked();
            });
        });

        egui::Window::new("Info")
            .open(&mut self.info_window_open)
            .show(ctx, |ui| {
                ui.label(format!("FPS: {:.3}", 1.0 / ts));
                ui.label(format!("Total Frame Time: {:.3}ms", ts * 1000.0));
                ui.label(format!(
                    "Fixed Update Time: {:.3}ms",
                    fixed_update_duration.as_secs_f32() * 1000.0
                ));
                ui.allocate_space(ui.available_size());
            });

        egui::Window::new("Settings")
            .open(&mut self.settings_window_open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Physics Enabled: ");
                    ui.checkbox(&mut self.physics_enabled, "");
                });
                ui.horizontal(|ui| {
                    ui.label("Gravity: ");
                    ui.add(
                        egui::DragValue::new(&mut self.gravity.x)
                            .speed(0.1)
                            .prefix("x: "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.gravity.y)
                            .speed(0.1)
                            .prefix("y: "),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Sweeping Colliders: ");
                    ui.checkbox(&mut self.sweeping_colliders, "");
                });
                ui.allocate_space(ui.available_size());
            });

        egui::Window::new("Quads")
            .open(&mut self.quads_window_open)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if ui.button("Add Quad").clicked() {
                        self.quads.push(Quad::default());
                    }
                    let mut quads_to_delete = vec![];
                    for (i, quad) in self.quads.iter_mut().enumerate() {
                        egui::CollapsingHeader::new(format!("Quad {i}")).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Position: ");
                                ui.add(
                                    egui::DragValue::new(&mut quad.position.x)
                                        .speed(0.1)
                                        .prefix("x: "),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut quad.position.y)
                                        .speed(0.1)
                                        .prefix("y: "),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("Velocity: ");
                                ui.add(
                                    egui::DragValue::new(&mut quad.velocity.x)
                                        .speed(0.1)
                                        .prefix("x: ")
                                        .suffix("m/s"),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut quad.velocity.y)
                                        .speed(0.1)
                                        .prefix("y: ")
                                        .suffix("m/s"),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("Rotation: ");
                                ui.drag_angle(&mut quad.rotation);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Angular Velocity: ");
                                // Copied from egui::ui::Ui::drag_angle
                                pub fn drag_angle(
                                    ui: &mut egui::Ui,
                                    radians: &mut f32,
                                ) -> egui::Response {
                                    let mut degrees = radians.to_degrees();
                                    let mut response = ui.add(
                                        egui::DragValue::new(&mut degrees).speed(1.0).suffix("°/s"),
                                    );

                                    // only touch `*radians` if we actually changed the degree value
                                    if degrees != radians.to_degrees() {
                                        *radians = degrees.to_radians();
                                        response.changed = true;
                                    }

                                    response
                                }
                                drag_angle(ui, &mut quad.angular_velocity);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Scale: ");
                                ui.add(
                                    egui::DragValue::new(&mut quad.scale.x)
                                        .speed(0.1)
                                        .prefix("x: "),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut quad.scale.y)
                                        .speed(0.1)
                                        .prefix("y: "),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("Color: ");
                                let mut rgb = quad.color.into();
                                egui::color_picker::color_edit_button_rgb(ui, &mut rgb);
                                quad.color = rgb.into();
                            });
                            ui.horizontal(|ui| {
                                ui.label("Dynamic: ");
                                ui.checkbox(&mut quad.dynamic, "");
                            });
                            if ui.button("Delete").clicked() {
                                quads_to_delete.push(i);
                            }
                        });
                    }

                    // not sure that this sort is 100% nessaseary, they should be added in the order of the for loop
                    quads_to_delete.sort();
                    // iterate backwards so that the indices dont get moved while removing
                    for quad in quads_to_delete.into_iter().rev() {
                        self.quads.remove(quad);
                    }

                    ui.allocate_space(ui.available_size());
                });
            });

        let egui::InnerResponse {
            inner: (rect, response),
            ..
        } = egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(51, 51, 51)))
            .show(ctx, |ui| {
                let size = ui.available_size();
                let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());

                let camera = CameraUniform {
                    position: self.camera.position,
                    rotation: self.camera.rotation,
                    zoom: self.camera.zoom,
                    screen_size: (size.x, size.y).into(),
                };
                let quads = self
                    .quads
                    .iter()
                    .map(|quad| StorageBufferQuad {
                        position: quad.position,
                        scale: quad.scale,
                        color: quad.color,
                        rotation: quad.rotation,
                    })
                    .collect::<Vec<_>>();
                ui.painter().add(egui::PaintCallback {
                    rect,
                    callback: Arc::new(
                        eframe::egui_wgpu::CallbackFn::new()
                            .prepare(move |device, queue, encoder, data| {
                                let renderer: &mut Renderer = data.get_mut().unwrap();
                                renderer.prepare(camera, &quads, device, queue, encoder)
                            })
                            .paint(move |_info, render_pass, data| {
                                let renderer: &Renderer = data.get().unwrap();
                                renderer.paint(render_pass)
                            }),
                    ),
                });
                (rect, response)
            });

        {
            let aspect = rect.width() / rect.height();

            if response.dragged() {
                let movement = response.drag_delta()
                    / self.camera.zoom
                    / (rect.size() * egui::vec2(0.5 / aspect, 0.5));
                self.camera.position.x += -movement.x;
                self.camera.position.y -= -movement.y;
            }

            if response.hovered() {
                ctx.input(|i| {
                    let old_zoom = self.camera.zoom;

                    if i.any_touches() {
                        self.camera.zoom *= i.zoom_delta();
                    }
                    if i.scroll_delta.y > 0.0 {
                        self.camera.zoom /= 0.9;
                    } else if i.scroll_delta.y < 0.0 {
                        self.camera.zoom *= 0.9;
                    }

                    let Some(cursor_pos) = i.pointer.hover_pos() else { return; };

                    let movement = (cursor_pos - rect.center())
                        * ((old_zoom - self.camera.zoom) / old_zoom)
                        / self.camera.zoom
                        / (rect.size() * egui::vec2(0.5 / aspect, 0.5));
                    self.camera.position.x += -movement.x;
                    self.camera.position.y -= -movement.y;
                });
            }
        }

        if !ctx.wants_keyboard_input() {
            ctx.input(|i| {
                if i.key_pressed(egui::Key::Space) {
                    self.physics_enabled = !self.physics_enabled;
                }
            });
        }

        if self.physics_enabled {
            ctx.request_repaint();
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("App", serde_json::to_string(self).unwrap());
        storage.flush();
    }
}
