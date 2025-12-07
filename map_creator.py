import tkinter as tk
from tkinter import filedialog, messagebox

# WARNING: this was entirely produced by an LLM, and not checked for correctness.
# This is only a helper tool to quickly develop new maps. You have been warned.


class MapEditor:
    def __init__(self, root):
        self.root = root
        self.root.title("2D Map Builder")

        # Data Storage
        self.walls = []  # Stores dicts: {'id': canvas_id, 'min': (x,y), 'max': (x,y)}
        self.spawns = []  # Stores dicts: {'id': canvas_id, 'pos': (x,y)}

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
        self.canvas_frame = tk.Frame(root, bg="#333333")
        self.canvas_frame.pack(side=tk.RIGHT, fill=tk.BOTH, expand=True)

        # Scrollbars
        self.h_scroll = tk.Scrollbar(self.canvas_frame, orient=tk.HORIZONTAL)
        self.v_scroll = tk.Scrollbar(self.canvas_frame, orient=tk.VERTICAL)

        self.canvas = tk.Canvas(
            self.canvas_frame,
            bg="white",
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

        # Initialize
        self.update_canvas_size()

    def set_tool(self):
        self.current_tool = self.tool_var.get()

    def update_canvas_size(self):
        try:
            w = int(self.entry_width.get())
            h = int(self.entry_height.get())
            self.canvas.config(scrollregion=(0, 0, w, h))
            # Draw a border guide
            self.canvas.delete("border")
            self.canvas.create_rectangle(
                0, 0, w, h, outline="red", width=2, tags="border"
            )
        except ValueError:
            messagebox.showerror("Error", "Width and Height must be integers.")

    def on_mouse_down(self, event):
        # Convert window coords to canvas coords (handling scroll)
        canvas_x = self.canvas.canvasx(event.x)
        canvas_y = self.canvas.canvasy(event.y)

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
            # Create a spawn point (circle)
            r = 10  # radius
            item_id = self.canvas.create_oval(
                canvas_x - r,
                canvas_y - r,
                canvas_x + r,
                canvas_y + r,
                fill="#00ff00",
                outline="black",
            )
            # Add text label
            text_id = self.canvas.create_text(canvas_x, canvas_y, text="S")

            self.spawns.append(
                {
                    "ids": [item_id, text_id],  # Store both visual elements
                    "pos": (canvas_x, canvas_y),
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

            # Store the wall data
            self.walls.append(
                {"id": self.current_rect, "min": (x1, y1), "max": (x2, y2)}
            )
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
            self.update_canvas_size()  # Redraw border

    def export_map(self):
        # Generate the string
        w = self.entry_width.get()
        h = self.entry_height.get()

        output = []

        # 1. Map Definition
        output.append("let map = MapDefinition {")
        output.append(f"    width: {float(w):.1f},")
        output.append(f"    height: {float(h):.1f},")
        output.append("    walls: vec![")

        for wall in self.walls:
            output.append("        RectWall {")
            output.append(
                f"            min: ({wall['min'][0]:.1f}, {wall['min'][1]:.1f}).into(),"
            )
            output.append(
                f"            max: ({wall['max'][0]:.1f}, {wall['max'][1]:.1f}).into(),"
            )
            output.append("        },")

        output.append("    ],")
        output.append("};")

        output.append("")  # Spacer

        # 2. Spawn Points
        output.append(f"// Spawn Points count: {len(self.spawns)}")
        output.append("let spawn_points = vec![")
        for spawn in self.spawns:
            output.append(f"    ({spawn['pos'][0]:.1f}, {spawn['pos'][1]:.1f}),")
        output.append("];")

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
            with open(file_path, "w") as f:
                f.write(full_text)
            messagebox.showinfo("Success", f"Map saved to {file_path}")


if __name__ == "__main__":
    root = tk.Tk()
    root.geometry("1000x700")
    app = MapEditor(root)
    root.mainloop()
