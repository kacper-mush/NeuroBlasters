import tkinter as tk
from tkinter import filedialog, messagebox, ttk

# WARNING: this was entirely produced by an LLM, and not checked for correctness.
# This is only a helper tool to quickly develop new maps. You have been warned.


class MapEditor:
    def __init__(self, root):
        self.root = root
        self.root.title("2D Map Builder")

        # Data Storage
        self.walls = []  # Stores dicts: {'id': canvas_id, 'min': (x,y), 'max': (x,y)}
        # Updated spawn storage: {'ids': [canvas_ids], 'pos': (x,y), 'team': 'Red'|'Blue'}
        self.spawns = []

        self.current_tool = "wall"  # "wall" or "spawn"
        self.start_x = None
        self.start_y = None
        self.current_rect = None

        # --- UI Layout ---

        # Control Panel (Left Side)
        self.controls_frame = tk.Frame(root, width=200, bg="#f0f0f0", padx=10, pady=10)
        self.controls_frame.pack(side=tk.LEFT, fill=tk.Y)

        # Map Size Inputs
        tk.Label(self.controls_frame, text="Map Width:").pack(anchor="w")
        self.entry_width = tk.Entry(self.controls_frame)
        self.entry_width.insert(0, "800")
        self.entry_width.pack(fill=tk.X)

        tk.Label(self.controls_frame, text="Map Height:").pack(anchor="w", pady=(5, 0))
        self.entry_height = tk.Entry(self.controls_frame)
        self.entry_height.insert(0, "600")
        self.entry_height.pack(fill=tk.X)

        self.btn_resize = tk.Button(
            self.controls_frame, text="Set Canvas Size", command=self.update_canvas_size
        )
        self.btn_resize.pack(fill=tk.X, pady=10)

        # Tools
        tk.Label(self.controls_frame, text="Tools:", font=("Arial", 10, "bold")).pack(
            anchor="w", pady=(10, 5)
        )

        self.tool_var = tk.StringVar(value="wall")

        rb_wall = tk.Radiobutton(
            self.controls_frame,
            text="Draw Walls (Drag)",
            variable=self.tool_var,
            value="wall",
            command=self.set_tool,
        )
        rb_wall.pack(anchor="w")

        rb_spawn = tk.Radiobutton(
            self.controls_frame,
            text="Place Spawn (Click)",
            variable=self.tool_var,
            value="spawn",
            command=self.set_tool,
        )
        rb_spawn.pack(anchor="w")

        # --- Team Selection for Spawn Points ---
        tk.Label(
            self.controls_frame,
            text="Spawn Team:",
            font=("Arial", 10, "bold"),
            fg="darkblue",
        ).pack(anchor="w", pady=(10, 5))

        self.team_var = tk.StringVar(value="Red")
        self.team_combobox = ttk.Combobox(
            self.controls_frame,
            textvariable=self.team_var,
            values=("Red", "Blue"),
            state="readonly",
        )
        self.team_combobox.pack(fill=tk.X)
        # --- END NEW UI ---

        # Actions
        self.btn_undo = tk.Button(
            self.controls_frame, text="Undo Last", command=self.undo_last
        )
        self.btn_undo.pack(fill=tk.X, pady=(20, 5))

        self.btn_clear = tk.Button(
            self.controls_frame, text="Clear All", command=self.clear_all, bg="#ffcccc"
        )
        self.btn_clear.pack(fill=tk.X, pady=5)

        self.btn_export = tk.Button(
            self.controls_frame,
            text="EXPORT to File",
            command=self.export_map,
            bg="#ccffcc",
            height=2,
        )
        self.btn_export.pack(fill=tk.X, side=tk.BOTTOM)

        # Canvas Area (Right Side)
        # Use a contrasting background color for the canvas frame to visually center the map area
        self.canvas_frame = tk.Frame(root, bg="#333333")
        self.canvas_frame.pack(side=tk.RIGHT, fill=tk.BOTH, expand=True)

        # Scrollbars
        self.h_scroll = tk.Scrollbar(self.canvas_frame, orient=tk.HORIZONTAL)
        self.v_scroll = tk.Scrollbar(self.canvas_frame, orient=tk.VERTICAL)

        # The canvas now has a dark gray background, contrasting with the map drawing area
        self.canvas = tk.Canvas(
            self.canvas_frame,
            bg="#888888",  # Default canvas background color (the scrollable area)
            xscrollcommand=self.h_scroll.set,
            yscrollcommand=self.v_scroll.set,
        )

        self.h_scroll.config(command=self.canvas.xview)
        self.v_scroll.config(command=self.canvas.yview)

        self.h_scroll.pack(side=tk.BOTTOM, fill=tk.X)
        self.v_scroll.pack(side=tk.RIGHT, fill=tk.Y)
        self.canvas.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)

        # Bind Mouse Events
        self.canvas.bind("<ButtonPress-1>", self.on_mouse_down)
        self.canvas.bind("<B1-Motion>", self.on_mouse_drag)
        self.canvas.bind("<ButtonRelease-1>", self.on_mouse_up)

        # Bind the configure event to recenter the view when the window is resized
        self.canvas.bind("<Configure>", self.recenter_viewport)

        # Initialize
        self.update_canvas_size()

    def set_tool(self):
        self.current_tool = self.tool_var.get()

    def recenter_viewport(self, event=None):
        """Adjusts the viewport to try and center the scrollregion if the canvas is larger."""

        try:
            map_w = int(self.entry_width.get())
            map_h = int(self.entry_height.get())

            canvas_w = self.canvas.winfo_width()
            canvas_h = self.canvas.winfo_height()

            # --- Horizontal Centering ---
            if canvas_w > map_w:
                # Calculate the fraction needed to position the map's start (0, 0)
                # such that it is visually centered.
                # Total unused space = canvas_w - map_w
                # Half unused space (the offset) = (canvas_w - map_w) / 2

                # The scroll unit is the total scrollable range (map_w).
                # The fraction to move is: offset / map_w

                x_fraction = ((canvas_w - map_w) / 2) / map_w
                # xview_moveto expects a fraction (0.0 to 1.0)
                # To move the view to the left (showing content that starts "earlier"),
                # we need a negative fraction.
                self.canvas.xview_moveto(-x_fraction)
            else:
                # If map is larger than canvas, just align to the start (no centering)
                self.canvas.xview_moveto(0.0)

            # --- Vertical Centering ---
            if canvas_h > map_h:
                y_fraction = ((canvas_h - map_h) / 2) / map_h
                self.canvas.yview_moveto(-y_fraction)
            else:
                self.canvas.yview_moveto(0.0)

        except (ValueError, ZeroDivisionError):
            # Ignore if size entries are not valid integers or zero
            pass

    def update_canvas_size(self):
        try:
            w = int(self.entry_width.get())
            h = int(self.entry_height.get())

            # Set the scroll region to the size of the map (0, 0, w, h).
            # The centering logic in recenter_viewport will handle the offsets.
            self.canvas.config(scrollregion=(0, 0, w, h))

            # Draw a visual container (Map Area) inside the larger scrollable canvas.
            self.canvas.delete("map_area")
            map_area_id = self.canvas.create_rectangle(
                0, 0, w, h, fill="white", outline="black", width=1, tags=("map_area")
            )

            # Draw a border guide
            self.canvas.delete("border")
            self.canvas.create_rectangle(
                0, 0, w, h, outline="red", width=2, tags="border"
            )

            # Re-draw all existing walls and spawns to be on top of the new map_area background
            self.redraw_objects()

            # Call recenter function to position the map drawing in the center of the viewport
            # Use self.root.after(10, ...) to ensure the canvas has updated its size (winfo_width)
            self.root.after(10, self.recenter_viewport)

        except ValueError:
            messagebox.showerror("Error", "Width and Height must be integers.")

    def redraw_objects(self):
        """Utility to redraw existing objects after canvas changes."""
        # Collect and redraw walls
        temp_walls = self.walls[:]
        self.walls = []
        for wall in temp_walls:
            self.canvas.delete(wall["id"])
            x1, y1 = wall["min"]
            x2, y2 = wall["max"]
            rect_id = self.canvas.create_rectangle(
                x1, y1, x2, y2, outline="black", fill="gray"
            )
            self.walls.append({"id": rect_id, "min": (x1, y1), "max": (x2, y2)})

        # Collect and redraw spawns
        temp_spawns = self.spawns[:]
        self.spawns = []
        for spawn in temp_spawns:
            for visual_id in spawn["ids"]:
                self.canvas.delete(visual_id)

            canvas_x, canvas_y = spawn["pos"]
            team = spawn["team"]

            r = 10  # radius
            color = "red" if team == "Red" else "blue"

            item_id = self.canvas.create_oval(
                canvas_x - r,
                canvas_y - r,
                canvas_x + r,
                canvas_y + r,
                fill=color,
                outline="black",
            )
            text_id = self.canvas.create_text(
                canvas_x, canvas_y, text=team[0].upper(), fill="white"
            )
            self.spawns.append(
                {
                    "ids": [item_id, text_id],
                    "pos": (canvas_x, canvas_y),
                    "team": team,
                }
            )

    def on_mouse_down(self, event):
        # Convert window coords to canvas coords (handling scroll)
        canvas_x = self.canvas.canvasx(event.x)
        canvas_y = self.canvas.canvasy(event.y)

        # Check if click is outside map boundaries
        try:
            map_w = int(self.entry_width.get())
            map_h = int(self.entry_height.get())
            if not (0 <= canvas_x <= map_w and 0 <= canvas_y <= map_h):
                # Ignore click if outside the defined map area
                return
        except ValueError:
            return

        if self.current_tool == "wall":
            self.start_x = canvas_x
            self.start_y = canvas_y
            # Create a temporary rectangle
            self.current_rect = self.canvas.create_rectangle(
                self.start_x,
                self.start_y,
                self.start_x,
                self.start_y,
                outline="black",
                fill="gray",
                stipple="gray50",
            )
        elif self.current_tool == "spawn":
            team = self.team_var.get()
            color = "red" if team == "Red" else "blue"

            # Create a spawn point (circle)
            r = 10  # radius
            item_id = self.canvas.create_oval(
                canvas_x - r,
                canvas_y - r,
                canvas_x + r,
                canvas_y + r,
                fill=color,  # Use team color
                outline="black",
            )
            # Add text label (R or B)
            text_id = self.canvas.create_text(
                canvas_x, canvas_y, text=team[0].upper(), fill="white"  # 'R' or 'B'
            )

            self.spawns.append(
                {
                    "ids": [item_id, text_id],  # Store both visual elements
                    "pos": (canvas_x, canvas_y),
                    "team": team,  # Store the team
                }
            )

    def on_mouse_drag(self, event):
        if self.current_tool == "wall" and self.current_rect:
            cur_x = self.canvas.canvasx(event.x)
            cur_y = self.canvas.canvasy(event.y)
            self.canvas.coords(
                self.current_rect, self.start_x, self.start_y, cur_x, cur_y
            )

    def on_mouse_up(self, event):
        if self.current_tool == "wall" and self.current_rect:
            cur_x = self.canvas.canvasx(event.x)
            cur_y = self.canvas.canvasy(event.y)

            # Normalize coordinates (ensure min is always top-left)
            x1, x2 = sorted([self.start_x, cur_x])
            y1, y2 = sorted([self.start_y, cur_y])

            # Clamp coordinates to map boundaries
            try:
                map_w = int(self.entry_width.get())
                map_h = int(self.entry_height.get())

                x1 = max(0.0, min(x1, map_w))
                x2 = max(0.0, min(x2, map_w))
                y1 = max(0.0, min(y1, map_h))
                y2 = max(0.0, min(y2, map_h))
            except ValueError:
                # If width/height are invalid, don't clamp
                pass

            # Update the temporary rectangle's final position with clamped/normalized values
            self.canvas.coords(self.current_rect, x1, y1, x2, y2)

            # Store the wall data
            self.walls.append(
                {"id": self.current_rect, "min": (x1, y1), "max": (x2, y2)}
            )

            # Ensure the rectangle is drawn solid now that drawing is complete
            self.canvas.itemconfig(self.current_rect, fill="gray", stipple="")
            self.current_rect = None

    def undo_last(self):
        # Determine which list to pop from based on what was added last?
        # For simplicity, we undo based on the currently selected tool,
        # or we could track a global history. Let's try global history simply:
        # Actually, let's just undo based on current tool mode for specific utility.

        if self.current_tool == "wall" and self.walls:
            last = self.walls.pop()
            self.canvas.delete(last["id"])
        elif self.current_tool == "spawn" and self.spawns:
            last = self.spawns.pop()
            for visual_id in last["ids"]:
                self.canvas.delete(visual_id)
        else:
            # If nothing in current tool, try the other just in case user forgot
            if self.walls:
                last = self.walls.pop()
                self.canvas.delete(last["id"])
            elif self.spawns:
                last = self.spawns.pop()
                for visual_id in last["ids"]:
                    self.canvas.delete(visual_id)

    def clear_all(self):
        if messagebox.askyesno("Confirm", "Clear entire map?"):
            self.canvas.delete("all")
            self.walls = []
            self.spawns = []
            self.update_canvas_size()  # Redraw border and map area

    def export_map(self):
        # Generate the string
        w = self.entry_width.get()
        h = self.entry_height.get()

        output = []

        # Start Map Definition
        output.append("MapDefinition {")
        output.append(f"    width: {float(w):.1f},")
        output.append(f"    height: {float(h):.1f},")
        output.append("    walls: vec![")

        # Walls
        for wall in self.walls:
            output.append("        RectWall {")
            output.append(
                f"            min: ({wall['min'][0]:.1f}, {wall['min'][1]:.1f}).into(),"
            )
            output.append(
                f"            max: ({wall['max'][0]:.1f}, {wall['max'][1]:.1f}).into(),"
            )
            output.append("        },")

        output.append("    ],")  # End walls vec

        # Spawn Points
        output.append("    spawn_points: vec![")
        for spawn in self.spawns:
            # Team must be capitalized like the enum: Team::Red, Team::Blue
            team_str = f"Team::{spawn['team']}"
            output.append(
                f"        ({team_str}, ({spawn['pos'][0]:.1f}, {spawn['pos'][1]:.1f}).into()),"
            )
        output.append("    ],")  # End spawn_points vec

        output.append("}")  # End MapDefinition struct

        full_text = "\n".join(output)

        # Save to file
        file_path = filedialog.asksaveasfilename(
            defaultextension=".rs",
            filetypes=[
                ("Rust File", "*.rs"),
                ("Text File", "*.txt"),
                ("All Files", "*.*"),
            ],
        )

        if file_path:
            try:
                with open(file_path, "w") as f:
                    f.write(full_text)
                messagebox.showinfo("Success", f"Map saved to {file_path}")
            except Exception as e:
                messagebox.showerror("Error", f"Failed to save file: {e}")


if __name__ == "__main__":
    root = tk.Tk()
    root.geometry("1000x700")
    app = MapEditor(root)
    root.mainloop()
