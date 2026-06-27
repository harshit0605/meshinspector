mod offset_contours;
mod ply;
mod svg;
pub use offset_contours::{
    offset_contours, offset_contours_with_options, offset_contours_with_options_and_origins,
    offset_contours_with_options_and_origins_and_z_callback,
    offset_contours_with_options_and_origins_and_z_options,
    offset_contours_with_options_and_z_callback, offset_contours_with_options_and_z_options,
    offset_contours_with_variable_offsets, offset_contours_with_variable_offsets_and_origins,
    offset_contours_with_variable_offsets_and_origins_and_z_callback,
    offset_contours_with_variable_offsets_and_origins_and_z_options,
    offset_contours_with_variable_offsets_and_z_callback,
    offset_contours_with_variable_offsets_and_z_options, OffsetContourIndex,
    OffsetContoursCornerType, OffsetContoursEndType, OffsetContoursMode, OffsetContoursOptions,
    OffsetContoursOrigin, OffsetContoursResult, OffsetContoursZOptions, OffsetContoursZRestoreMode,
};
pub use ply::{object_lines_from_ply, object_lines_to_ply};
pub use svg::object_lines_from_svg;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectLinesColoringType {
    Solid,
    PerVertex,
    PerLine,
}

impl Default for ObjectLinesColoringType {
    fn default() -> Self {
        Self::Solid
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectLinesOptions {
    pub show_points: u32,
    pub smooth_connections: u32,
    pub line_width: f32,
    pub coloring_type: ObjectLinesColoringType,
}

impl Default for ObjectLinesOptions {
    fn default() -> Self {
        Self {
            show_points: 0,
            smooth_connections: 0,
            line_width: 1.0,
            coloring_type: ObjectLinesColoringType::Solid,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectLinesDocument {
    pub points: Vec<[f64; 3]>,
    pub lines: Vec<[usize; 2]>,
    pub show_points: u32,
    pub smooth_connections: u32,
    pub line_width: f32,
    pub coloring_type: ObjectLinesColoringType,
    pub line_colors: Vec<[u8; 4]>,
    pub vert_colors: Vec<[u8; 4]>,
    pub uv_coords: Vec<[f64; 2]>,
    pub texture_files: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MrLinesHalfEdgeRecord {
    next: i32,
    org: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MrLinesTopology {
    edges: Vec<MrLinesHalfEdgeRecord>,
    edge_per_vertex: Vec<i32>,
}

impl Default for ObjectLinesDocument {
    fn default() -> Self {
        let options = ObjectLinesOptions::default();
        Self {
            points: Vec::new(),
            lines: Vec::new(),
            show_points: options.show_points,
            smooth_connections: options.smooth_connections,
            line_width: options.line_width,
            coloring_type: options.coloring_type,
            line_colors: Vec::new(),
            vert_colors: Vec::new(),
            uv_coords: Vec::new(),
            texture_files: Vec::new(),
        }
    }
}

pub fn object_lines_from_contours(
    contours: &[Vec<[f64; 3]>],
    options: ObjectLinesOptions,
) -> Result<ObjectLinesDocument, String> {
    validate_options(&options)?;
    let mut document = ObjectLinesDocument {
        show_points: options.show_points,
        smooth_connections: options.smooth_connections,
        line_width: options.line_width,
        coloring_type: options.coloring_type,
        ..ObjectLinesDocument::default()
    };

    for contour in contours {
        if contour.len() < 2 {
            continue;
        }
        validate_contour(contour)?;
        let closed = contour.len() > 2 && contour.first() == contour.last();
        let point_count = contour.len() - usize::from(closed);
        if point_count < 2 {
            continue;
        }
        let start = document.points.len();
        document.points.extend_from_slice(&contour[..point_count]);
        for index in 0..point_count - 1 {
            document.lines.push([start + index, start + index + 1]);
        }
        if closed {
            document.lines.push([start + point_count - 1, start]);
        }
    }
    Ok(document)
}

pub fn object_lines_to_contours(
    document: &ObjectLinesDocument,
) -> Result<Vec<Vec<[f64; 3]>>, String> {
    validate_document(document)?;
    if document.lines.is_empty() {
        return Ok(Vec::new());
    }

    let mut adjacency = vec![Vec::<usize>::new(); document.points.len()];
    for (line_index, line) in document.lines.iter().enumerate() {
        adjacency[line[0]].push(line_index);
        adjacency[line[1]].push(line_index);
        if adjacency[line[0]].len() > 2 || adjacency[line[1]].len() > 2 {
            return Err("ObjectLines polyline vertices must have degree at most two".to_string());
        }
    }

    let mut used = vec![false; document.lines.len()];
    let mut contours = Vec::new();
    for line_index in 0..document.lines.len() {
        if used[line_index] {
            continue;
        }
        let line = document.lines[line_index];
        let start = if adjacency[line[0]].len() == 1 {
            line[0]
        } else if adjacency[line[1]].len() == 1 {
            line[1]
        } else {
            line[0]
        };
        contours.push(walk_component(
            start,
            &document.points,
            &document.lines,
            &adjacency,
            &mut used,
        )?);
    }
    Ok(contours)
}

pub fn object_lines_to_pts(document: &ObjectLinesDocument) -> Result<String, String> {
    let contours = object_lines_to_contours(document)?;
    let mut output = String::new();
    for contour in contours {
        output.push_str("BEGIN_Polyline\n");
        for point in contour {
            output.push_str(&format!(
                "{} {} {}\n",
                format_coordinate(point[0]),
                format_coordinate(point[1]),
                format_coordinate(point[2])
            ));
        }
        output.push_str("END_Polyline\n");
    }
    Ok(output)
}

pub fn object_lines_from_pts(source: &str) -> Result<ObjectLinesDocument, String> {
    let mut contours = Vec::new();
    let mut points = Vec::new();
    let mut in_polyline = false;

    for raw_line in source.lines() {
        let line = raw_line.trim_end();
        if !in_polyline {
            if line != "BEGIN_Polyline" {
                return Err("unsupported .PTS file with polylines".to_string());
            }
            in_polyline = true;
            continue;
        }

        if line == "END_Polyline" {
            in_polyline = false;
            if !points.is_empty() {
                contours.push(std::mem::take(&mut points));
            }
            continue;
        }

        points.push(parse_pts_point(line)?);
    }

    if in_polyline {
        return Err("unsupported .PTS file with polylines".to_string());
    }

    object_lines_from_contours(&contours, ObjectLinesOptions::default())
}

pub fn object_lines_to_dxf(document: &ObjectLinesDocument) -> Result<String, String> {
    let contours = object_lines_to_contours(document)?;
    let mut output = String::from("0\nSECTION\n2\nENTITIES\n");
    for contour in contours {
        if contour.is_empty() {
            continue;
        }
        output.push_str("0\nPOLYLINE\n8\n0\n66\n1\n");
        let flags = 8 + u32::from(contour.first() == contour.last());
        output.push_str(&format!("70\n{}\n", flags));
        for point in contour {
            output.push_str(&format!(
                "0\nVERTEX\n8\n0\n70\n32\n10\n{}\n20\n{}\n30\n{}\n",
                format_coordinate(point[0]),
                format_coordinate(point[1]),
                format_coordinate(point[2])
            ));
        }
        output.push_str("0\nSEQEND\n");
    }
    output.push_str("0\nENDSEC\n0\nEOF\n");
    Ok(output)
}

pub fn object_lines_to_mrlines(document: &ObjectLinesDocument) -> Result<Vec<u8>, String> {
    let contours = object_lines_to_contours(document)?;
    let (topology, points) = build_mrlines_topology(&contours)?;
    let mut output = Vec::new();
    push_u32(&mut output, topology.edges.len() as u32);
    for record in &topology.edges {
        push_i32(&mut output, record.next);
        push_i32(&mut output, record.org);
    }
    push_u32(&mut output, topology.edge_per_vertex.len() as u32);
    for edge in &topology.edge_per_vertex {
        push_i32(&mut output, *edge);
    }
    push_u32(&mut output, 3);
    push_u32(&mut output, points.len() as u32);
    for point in &points {
        push_f32(&mut output, point[0]);
        push_f32(&mut output, point[1]);
        push_f32(&mut output, point[2]);
    }
    Ok(output)
}

pub fn object_lines_from_mrlines(bytes: &[u8]) -> Result<ObjectLinesDocument, String> {
    let mut reader = MrLinesReader::new(bytes);
    let edge_count = reader.read_u32("Error reading topology from lines-file")? as usize;
    if edge_count % 2 != 0 {
        return Err("Error reading topology from lines-file".to_string());
    }
    let mut edges = Vec::with_capacity(edge_count);
    for _ in 0..edge_count {
        edges.push(MrLinesHalfEdgeRecord {
            next: reader.read_i32("Error reading topology from lines-file")?,
            org: reader.read_i32("Error reading topology from lines-file")?,
        });
    }
    let vertex_count = reader.read_u32("Error reading topology from lines-file")? as usize;
    let mut edge_per_vertex = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        edge_per_vertex.push(reader.read_i32("Error reading topology from lines-file")?);
    }
    validate_mrlines_topology(&edges, &edge_per_vertex)?;

    let point_type = reader.read_u32("Error reading the type of points from lines-file")?;
    if point_type != 3 {
        return Err("Unsupported point type in lines-file".to_string());
    }
    let point_count =
        reader.read_u32("Error reading the number of points from lines-file")? as usize;
    let mut points = Vec::with_capacity(point_count);
    for _ in 0..point_count {
        points.push([
            reader.read_f32("Error reading  points from lines-file")? as f64,
            reader.read_f32("Error reading  points from lines-file")? as f64,
            reader.read_f32("Error reading  points from lines-file")? as f64,
        ]);
    }

    let mut lines = Vec::new();
    for edge_index in (0..edges.len()).step_by(2) {
        let a = edges[edge_index].org;
        let b = edges[edge_index ^ 1].org;
        if a >= 0 && b >= 0 {
            if a as usize >= points.len() || b as usize >= points.len() {
                return Err("Error reading topology from lines-file".to_string());
            }
            lines.push([a as usize, b as usize]);
        }
    }
    let document = ObjectLinesDocument {
        points,
        lines,
        ..ObjectLinesDocument::default()
    };
    validate_document(&document)?;
    Ok(document)
}

fn walk_component(
    start: usize,
    points: &[[f64; 3]],
    lines: &[[usize; 2]],
    adjacency: &[Vec<usize>],
    used: &mut [bool],
) -> Result<Vec<[f64; 3]>, String> {
    let mut contour = vec![points[start]];
    let mut current = start;
    let max_steps = lines.len() + 1;
    for _ in 0..max_steps {
        let Some(line_index) = adjacency[current]
            .iter()
            .copied()
            .find(|line_index| !used[*line_index])
        else {
            return Ok(contour);
        };
        used[line_index] = true;
        let [a, b] = lines[line_index];
        let next = if a == current { b } else { a };
        contour.push(points[next]);
        current = next;
        if current == start {
            return Ok(contour);
        }
    }
    Err("ObjectLines component traversal did not terminate".to_string())
}

fn validate_options(options: &ObjectLinesOptions) -> Result<(), String> {
    if !options.line_width.is_finite() || options.line_width < 0.0 {
        return Err("ObjectLines line_width must be finite and non-negative".to_string());
    }
    Ok(())
}

pub(super) fn validate_contour(contour: &[[f64; 3]]) -> Result<(), String> {
    if contour
        .iter()
        .flatten()
        .any(|coordinate| !coordinate.is_finite())
    {
        return Err("ObjectLines contour coordinates must be finite".to_string());
    }
    Ok(())
}

pub(super) fn validate_document(document: &ObjectLinesDocument) -> Result<(), String> {
    validate_options(&ObjectLinesOptions {
        show_points: document.show_points,
        smooth_connections: document.smooth_connections,
        line_width: document.line_width,
        coloring_type: document.coloring_type,
    })?;
    if document
        .points
        .iter()
        .flatten()
        .any(|coordinate| !coordinate.is_finite())
    {
        return Err("ObjectLines point coordinates must be finite".to_string());
    }
    for line in &document.lines {
        if line[0] >= document.points.len() || line[1] >= document.points.len() {
            return Err("ObjectLines line references a missing point".to_string());
        }
        if line[0] == line[1] {
            return Err("ObjectLines line endpoints must be distinct".to_string());
        }
    }
    if !document.vert_colors.is_empty() && document.vert_colors.len() != document.points.len() {
        return Err("ObjectLines vertex colors must match point count".to_string());
    }
    if !document.uv_coords.is_empty() && document.uv_coords.len() != document.points.len() {
        return Err("ObjectLines UV coordinates must match point count".to_string());
    }
    if document
        .uv_coords
        .iter()
        .flatten()
        .any(|coordinate| !coordinate.is_finite())
    {
        return Err("ObjectLines UV coordinates must be finite".to_string());
    }
    if !document.line_colors.is_empty() && document.line_colors.len() != document.lines.len() {
        return Err("ObjectLines line colors must match line count".to_string());
    }
    Ok(())
}

fn parse_pts_point(line: &str) -> Result<[f64; 3], String> {
    let mut values = line.split_whitespace();
    let mut point = [0.0; 3];
    for (index, coordinate) in point.iter_mut().enumerate() {
        let Some(value) = values.next() else {
            return Err("unsupported .PTS file with polylines".to_string());
        };
        let parsed = if index == 2 {
            parse_pts_last_coordinate(value)?
        } else {
            value
                .parse::<f32>()
                .map_err(|_| "unsupported .PTS file with polylines".to_string())?
        };
        if !parsed.is_finite() {
            return Err("ObjectLines contour coordinates must be finite".to_string());
        }
        *coordinate = parsed as f64;
    }
    Ok(point)
}

fn parse_pts_last_coordinate(value: &str) -> Result<f32, String> {
    let bytes = value.as_bytes();
    let mut end = 0;
    if matches!(bytes.get(end), Some(b'+' | b'-')) {
        end += 1;
    }

    let mut digit_count = 0usize;
    while matches!(bytes.get(end), Some(byte) if byte.is_ascii_digit()) {
        end += 1;
        digit_count += 1;
    }

    if matches!(bytes.get(end), Some(b'.')) {
        end += 1;
        while matches!(bytes.get(end), Some(byte) if byte.is_ascii_digit()) {
            end += 1;
            digit_count += 1;
        }
    }

    if digit_count == 0 {
        return Err("unsupported .PTS file with polylines".to_string());
    }

    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        end += 1;
        if matches!(bytes.get(end), Some(b'+' | b'-')) {
            end += 1;
        }
        let exponent_start = end;
        while matches!(bytes.get(end), Some(byte) if byte.is_ascii_digit()) {
            end += 1;
        }
        if exponent_start == end {
            return Err("unsupported .PTS file with polylines".to_string());
        }
    }

    value[..end]
        .parse::<f32>()
        .map_err(|_| "unsupported .PTS file with polylines".to_string())
}

fn format_coordinate(value: f64) -> String {
    if value == 0.0 {
        "0".to_string()
    } else {
        format!("{}", value)
    }
}

fn build_mrlines_topology(
    contours: &[Vec<[f64; 3]>],
) -> Result<(MrLinesTopology, Vec<[f32; 3]>), String> {
    let mut topology = MrLinesTopology {
        edges: Vec::new(),
        edge_per_vertex: Vec::new(),
    };
    let mut points = Vec::new();
    for contour in contours {
        if contour.len() < 2 {
            continue;
        }
        let closed = contour.len() > 2 && contour.first() == contour.last();
        let e0 = topology.make_edge();
        let v0 = add_mrlines_point(&mut points, &mut topology, contour[0])?;
        topology.set_org(e0, v0)?;
        let mut edge = e0;
        for point in contour.iter().take(contour.len() - 1).skip(1) {
            let next_edge = topology.make_edge();
            topology.splice(next_edge, sym(edge))?;
            let vertex = add_mrlines_point(&mut points, &mut topology, *point)?;
            topology.set_org(next_edge, vertex)?;
            edge = next_edge;
        }
        if closed {
            topology.splice(e0, sym(edge))?;
        } else {
            let vertex = add_mrlines_point(&mut points, &mut topology, *contour.last().unwrap())?;
            topology.set_org(sym(edge), vertex)?;
        }
    }
    Ok((topology, points))
}

fn add_mrlines_point(
    points: &mut Vec<[f32; 3]>,
    topology: &mut MrLinesTopology,
    point: [f64; 3],
) -> Result<i32, String> {
    let converted = [
        f64_to_f32(point[0])?,
        f64_to_f32(point[1])?,
        f64_to_f32(point[2])?,
    ];
    let vertex = points.len() as i32;
    points.push(converted);
    topology.edge_per_vertex.push(-1);
    Ok(vertex)
}

fn f64_to_f32(value: f64) -> Result<f32, String> {
    let converted = value as f32;
    if !converted.is_finite() {
        return Err("ObjectLines point coordinates must fit MeshLib Vector3f".to_string());
    }
    Ok(converted)
}

impl MrLinesTopology {
    fn make_edge(&mut self) -> i32 {
        let first = self.edges.len() as i32;
        self.edges.push(MrLinesHalfEdgeRecord {
            next: first,
            org: -1,
        });
        self.edges.push(MrLinesHalfEdgeRecord {
            next: first + 1,
            org: -1,
        });
        first
    }

    fn set_org(&mut self, edge: i32, vertex: i32) -> Result<(), String> {
        let old_vertex = self.record(edge)?.org;
        if old_vertex == vertex {
            return Ok(());
        }
        self.set_org_ring(edge, vertex)?;
        if old_vertex >= 0 {
            self.edge_per_vertex[old_vertex as usize] = -1;
        }
        if vertex >= 0 {
            self.edge_per_vertex[vertex as usize] = edge;
        }
        Ok(())
    }

    fn set_org_ring(&mut self, edge: i32, vertex: i32) -> Result<(), String> {
        let mut current = edge;
        loop {
            let next = self.record(current)?.next;
            self.record_mut(current)?.org = vertex;
            current = next;
            if current == edge {
                break;
            }
        }
        Ok(())
    }

    fn splice(&mut self, a: i32, b: i32) -> Result<(), String> {
        if a == b {
            return Ok(());
        }
        let a_org = self.record(a)?.org;
        let b_org = self.record(b)?.org;
        if a_org != b_org {
            if a_org >= 0 {
                self.set_org_ring(b, a_org)?;
            } else if b_org >= 0 {
                self.set_org_ring(a, b_org)?;
            }
        }

        let a_next = self.record(a)?.next;
        let b_next = self.record(b)?.next;
        self.record_mut(a)?.next = b_next;
        self.record_mut(b)?.next = a_next;

        if a_org == b_org && self.record(b)?.org >= 0 {
            self.set_org_ring(b, -1)?;
            let current_a_org = self.record(a)?.org;
            if current_a_org >= 0 {
                self.edge_per_vertex[current_a_org as usize] = a;
            }
        }
        Ok(())
    }

    fn record(&self, edge: i32) -> Result<MrLinesHalfEdgeRecord, String> {
        self.edges
            .get(edge as usize)
            .copied()
            .ok_or_else(|| "Error reading topology from lines-file".to_string())
    }

    fn record_mut(&mut self, edge: i32) -> Result<&mut MrLinesHalfEdgeRecord, String> {
        self.edges
            .get_mut(edge as usize)
            .ok_or_else(|| "Error reading topology from lines-file".to_string())
    }
}

fn validate_mrlines_topology(
    edges: &[MrLinesHalfEdgeRecord],
    edge_per_vertex: &[i32],
) -> Result<(), String> {
    for record in edges {
        if record.next < 0 || record.next as usize >= edges.len() {
            return Err("Error reading topology from lines-file".to_string());
        }
        if record.org >= edge_per_vertex.len() as i32 {
            return Err("Error reading topology from lines-file".to_string());
        }
    }
    for edge in edge_per_vertex {
        if *edge >= edges.len() as i32 {
            return Err("Error reading topology from lines-file".to_string());
        }
    }
    Ok(())
}

struct MrLinesReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> MrLinesReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_u32(&mut self, error: &str) -> Result<u32, String> {
        let bytes = self.read_exact(4, error)?;
        Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
    }

    fn read_i32(&mut self, error: &str) -> Result<i32, String> {
        let bytes = self.read_exact(4, error)?;
        Ok(i32::from_le_bytes(bytes.try_into().unwrap()))
    }

    fn read_f32(&mut self, error: &str) -> Result<f32, String> {
        let bytes = self.read_exact(4, error)?;
        Ok(f32::from_le_bytes(bytes.try_into().unwrap()))
    }

    fn read_exact(&mut self, count: usize, error: &str) -> Result<&'a [u8], String> {
        if self.offset + count > self.bytes.len() {
            return Err(error.to_string());
        }
        let start = self.offset;
        self.offset += count;
        Ok(&self.bytes[start..self.offset])
    }
}

fn sym(edge: i32) -> i32 {
    edge ^ 1
}
fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}
fn push_i32(output: &mut Vec<u8>, value: i32) {
    output.extend_from_slice(&value.to_le_bytes());
}
fn push_f32(output: &mut Vec<u8>, value: f32) {
    output.extend_from_slice(&value.to_le_bytes());
}
