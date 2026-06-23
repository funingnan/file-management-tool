use rand::Rng;
use crate::models::*;
pub fn layout_graph(graph: &mut GraphData, width: f32, height: f32) {
    let mut rng = rand::thread_rng(); let cx = width / 2.0; let cy = height / 2.0;
    for n in &mut graph.nodes { if n.x == 0.0 && n.y == 0.0 { n.x = cx + rng.gen_range(-width*0.3..width*0.3); n.y = cy + rng.gen_range(-height*0.3..height*0.3); } }
    for _ in 0..100 {
        let n = graph.nodes.len(); if n == 0 { return; }
        let mut f = vec![(0.0f32, 0.0f32); n];
        for i in 0..n { for j in (i+1)..n { let dx=graph.nodes[i].x-graph.nodes[j].x; let dy=graph.nodes[i].y-graph.nodes[j].y; let d=(dx*dx+dy*dy).sqrt().max(1.0); let fo=5000.0/(d*d); let fx=dx/d*fo; let fy=dy/d*fo; f[i].0+=fx; f[i].1+=fy; f[j].0-=fx; f[j].1-=fy; } }
        for e in &graph.edges { if let (Some(fi),Some(ti)) = (graph.nodes.iter().position(|n|n.id==e.from), graph.nodes.iter().position(|n|n.id==e.to)) { let dx=graph.nodes[ti].x-graph.nodes[fi].x; let dy=graph.nodes[ti].y-graph.nodes[fi].y; let d=(dx*dx+dy*dy).sqrt().max(1.0); let fo=(d-100.0)*0.05*e.weight.sqrt(); let fx=dx/d*fo; let fy=dy/d*fo; f[fi].0+=fx; f[fi].1+=fy; f[ti].0-=fx; f[ti].1-=fy; } }
        for (i,n) in graph.nodes.iter().enumerate() { f[i].0 += (cx-n.x)*0.01; f[i].1 += (cy-n.y)*0.01; }
        for (i,n) in graph.nodes.iter_mut().enumerate() { n.vx=(n.vx+f[i].0)*0.6; n.vy=(n.vy+f[i].1)*0.6; n.x+=n.vx; n.y+=n.vy; n.x=n.x.clamp(30.0,width-30.0); n.y=n.y.clamp(30.0,height-30.0); }
    }
}
