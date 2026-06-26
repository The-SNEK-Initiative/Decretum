// IBM System/360 (32-bit) and z/Architecture (64-bit).
// 16 GPRs (R0-R15), 4 floating point registers (F0,F2,F4,F6), 2+4 PSW bits.
// Standard IBM mainframe instruction formats: RR, RX, RS, SI, SS, RRE, RRF, RXE, RSE, RIE.

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct S360BuildOutput { pub bin_path: PathBuf, pub bin_size: usize }
pub struct S360Builder;
pub struct ZArchBuildOutput { pub bin_path: PathBuf, pub bin_size: usize }
pub struct ZArchBuilder;

#[derive(Clone,Copy,PartialEq)]
enum MGpr { R0,R1,R2,R3,R4,R5,R6,R7,R8,R9,R10,R11,R12,R13,R14,R15 }

fn mrp(s:&str)->Option<MGpr>{let n=s.trim_start_matches('r').parse::<u8>().ok()?; if n<=15{Some(unsafe{std::mem::transmute(n)})}else{None}}
fn mrn(r:MGpr)->u8{r as u8}

#[derive(Clone)]
struct MInst{op:u8,op2:u8,r1:u8,r2:u8,x2:u8,b1:u8,b2:u8,d1:u16,d2:u16,i16:i16,il:u32,is_z:bool}

fn menc_360(i:&MInst) -> Vec<u8> {
    match i.op {
        0x1A..=0x1F|0xB2..=0xB5|0xB9..=0xBF|0xE3|0xEB|0xE5 => { // RR/RRE
            let mut v=vec![i.op]; v.push(i.r1<<4|i.r2); v
        }
        0x50..=0x5F|0x40..=0x4F => { // RX
            let mut v=vec![i.op,i.r1<<4|i.x2,i.b1]; v.extend(&i.d1.to_be_bytes()); v
        }
        0x80..=0x8F|0x90..=0x9F|0xA0..=0xAF => { // RS/SI
            let mut v=vec![i.op,i.r1<<4|i.r2,i.b1]; v.extend(&i.d1.to_be_bytes()); v
        }
        _ => { // SS or default
            let mut v=vec![i.op,i.r1<<4|i.r2,i.b1]; v.extend(&i.d1.to_be_bytes()[..2]); v.push(i.b2); v.extend(&i.d2.to_be_bytes()[..2]); v
        }
    }
}
fn menc_zarch(i:&MInst) -> Vec<u8> {
    let mut base=menc_360(i); if i.is_z&&i.op>=0xE0{base.extend(&40u32.to_be_bytes()[..2]);} base
}

fn mlower(t:&str,is_z:bool)->Result<MInst,String>{
    let t=t.trim(); if t.is_empty()||t.starts_with(';'){return Err("".into())}
    let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
    if parts.is_empty(){return Err("".into())} let m=parts[0];
    let joined=parts[1..].join(" "); let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
    let gr=|s:&str| mrp(s).ok_or_else(||format!("bad r'{}'",s));
    Ok(match m{
        "lr"|"mov" if args.len()==2=>MInst{op:0x18,op2:0,r1:mrn(gr(args[1])?),r2:mrn(gr(args[0])?),x2:0,b1:0,b2:0,d1:0,d2:0,i16:0,il:0,is_z},
        "ar"|"add" if args.len()==2=>MInst{op:0x1A,op2:0,r1:mrn(gr(args[0])?),r2:mrn(gr(args[1])?),x2:0,b1:0,b2:0,d1:0,d2:0,i16:0,il:0,is_z},
        "sr"|"sub" if args.len()==2=>MInst{op:0x1B,op2:0,r1:mrn(gr(args[0])?),r2:mrn(gr(args[1])?),x2:0,b1:0,b2:0,d1:0,d2:0,i16:0,il:0,is_z},
        "mr"|"mul" if args.len()==2=>MInst{op:0x1C,op2:0,r1:mrn(gr(args[0])?),r2:mrn(gr(args[1])?),x2:0,b1:0,b2:0,d1:0,d2:0,i16:0,il:0,is_z},
        "dr"|"div" if args.len()==2=>MInst{op:0x1D,op2:0,r1:mrn(gr(args[0])?),r2:mrn(gr(args[1])?),x2:0,b1:0,b2:0,d1:0,d2:0,i16:0,il:0,is_z},
        "a" if args.len()==3=>MInst{op:0x5A,op2:0,r1:mrn(gr(args[0])?),r2:0,x2:mrn(gr(args[1])?),b1:mrn(gr(args[2])?),b2:0,d1:0,d2:0,i16:0,il:0,is_z},
        "s" if args.len()==3=>MInst{op:0x5B,op2:0,r1:mrn(gr(args[0])?),r2:0,x2:mrn(gr(args[1])?),b1:mrn(gr(args[2])?),b2:0,d1:0,d2:0,i16:0,il:0,is_z},
        "mvc" if args.len()==4=>{
            let l=args[3].parse::<u8>().unwrap_or(0);
            MInst{op:0xD2,op2:l,r1:mrn(gr(args[0])?),r2:0,x2:0,b1:mrn(gr(args[1])?),b2:mrn(gr(args[2])?),d1:0,d2:0,i16:0,il:0,is_z}
        }
        "bc"|"jmp" if args.len()==1=>MInst{op:0x47,op2:0,r1:0xF,r2:0,x2:0,b1:0,b2:0,d1:0,d2:0,i16:0,il:0,is_z},
        "balr"|"call" if args.len()==1=>MInst{op:0x05,op2:0,r1:mrn(gr(args[0])?),r2:0,x2:0,b1:0,b2:0,d1:0,d2:0,i16:0,il:0,is_z},
        "br"|"ret"=>MInst{op:0x07,op2:0,r1:0xF,r2:0,x2:0,b1:0,b2:0,d1:0,d2:0,i16:0,il:0,is_z},
        "sll" if args.len()==2=>MInst{op:0x89,op2:0,r1:mrn(gr(args[0])?),r2:args[1].parse::<u8>().unwrap_or(0),x2:0,b1:0,b2:0,d1:0,d2:0,i16:0,il:0,is_z},
        "srl" if args.len()==2=>MInst{op:0x88,op2:0,r1:mrn(gr(args[0])?),r2:args[1].parse::<u8>().unwrap_or(0),x2:0,b1:0,b2:0,d1:0,d2:0,i16:0,il:0,is_z},
        "nop"=>MInst{op:0x07,op2:0,r1:0,r2:0,x2:0,b1:0,b2:0,d1:0,d2:0,i16:0,il:0,is_z},
        _=>return Err(format!("unknown s360/zar '{}'",m)),
    })
}

fn mbuild(p:&Program,is_z:bool)->Result<Vec<u8>,String>{
    let mut bin=Vec::new();
    struct CfFrame{kind:CfKind,endif_label:String,else_label:String,br_indices:Vec<usize>,has_else:bool,start_pos:usize}
    enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:u32=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim(); if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t=="ret"{bin.extend(menc_360(&MInst{op:0x07,op2:0,r1:0xF,r2:0,x2:0,b1:0,b2:0,d1:0,d2:0,i16:0,il:0,is_z}));continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(menc_360(&MInst{op:0x05,op2:0,r1:0xF,r2:0xF,x2:0,b1:0,b2:0,d1:0,d2:0,i16:0,il:0,is_z}));continue}
        if let Some(r)=t.strip_prefix("if "){
            let rn=r.trim().trim_start_matches('r').parse::<u8>().unwrap_or(0);
            cf_counter+=1;
            bin.extend(&[0x12,(rn<<4)|rn]); // LTR rN, rN
            let p=bin.len();bin.extend(&[0x47,0x80,0x00,0x00,0x00]); // BC 8, d1
            cf_stack.push(CfFrame{kind:CfKind::If,endif_label:format!("_cf{}",cf_counter),else_label:format!("_cf{}_else",cf_counter),br_indices:vec![p],has_else:false,start_pos:0});
            continue;
        }
        if let Some(r)=t.strip_prefix("elif "){
            let f=cf_stack.last_mut().ok_or("elif without if")?;
            if f.has_else{return Err("elif after else".into())}
            let prev=f.br_indices.pop().ok_or("internal")?;
            let addr=bin.len() as u16;
            let ba=addr.to_be_bytes();bin[prev+3]=ba[0];bin[prev+4]=ba[1];
            let jmp_idx=bin.len();bin.extend(&[0x47,0xF0,0x00,0x00,0x00]);
            f.br_indices.push(jmp_idx);
            let rn=r.trim().trim_start_matches('r').parse::<u8>().unwrap_or(0);
            bin.extend(&[0x12,(rn<<4)|rn]);
            let p=bin.len();bin.extend(&[0x47,0x80,0x00,0x00,0x00]);
            f.br_indices.push(p);
            continue;
        }
        if t=="else"{
            let f=cf_stack.last_mut().ok_or("else without if")?;
            if f.has_else{return Err("duplicate else".into())}
            f.has_else=true;
            let prev=f.br_indices.pop().ok_or("internal")?;
            let addr=bin.len() as u16;
            let ba=addr.to_be_bytes();bin[prev+3]=ba[0];bin[prev+4]=ba[1];
            let jmp_idx=bin.len();bin.extend(&[0x47,0xF0,0x00,0x00,0x00]);
            f.br_indices.push(jmp_idx);
            continue;
        }
        if t=="endif"{
            let f=cf_stack.pop().ok_or("endif without if/while")?;
            if !matches!(f.kind,CfKind::If){return Err("endif without matching if".into())}
            let addr=bin.len() as u16;
            for &idx in &f.br_indices{
                let ba=addr.to_be_bytes();
                bin[idx+3]=ba[0];bin[idx+4]=ba[1];
            }
            continue;
        }
        if let Some(r)=t.strip_prefix("while "){
            let rn=r.trim().trim_start_matches('r').parse::<u8>().unwrap_or(0);
            let sp=bin.len();cf_counter+=1;
            bin.extend(&[0x12,(rn<<4)|rn]);
            let p=bin.len();bin.extend(&[0x47,0x80,0x00,0x00,0x00]);
            cf_stack.push(CfFrame{kind:CfKind::While,endif_label:format!("_cf{}",cf_counter),else_label:String::new(),br_indices:vec![p],has_else:false,start_pos:sp});
            continue;
        }
        if t=="endwhile"{
            let f=cf_stack.pop().ok_or("endwhile without while")?;
            if !matches!(f.kind,CfKind::While){return Err("endwhile without matching while".into())}
            let addr=f.start_pos as u16;
            let ba=addr.to_be_bytes();
            bin.extend(&[0x47,0xF0,0x00,ba[0],ba[1]]);
            let addr=bin.len() as u16;
            for &idx in &f.br_indices{
                let ba=addr.to_be_bytes();
                bin[idx+3]=ba[0];bin[idx+4]=ba[1];
            }
            continue;
        }
        if let Ok(i)=mlower(t,is_z){bin.extend(if is_z{menc_zarch(&i)}else{menc_360(&i)});}
    }}
    if !cf_stack.is_empty(){return Err("unclosed if/while block".into())}
    Ok(bin)
}

impl S360Builder{pub fn build_bin(p:&Program,out:&Path)->Result<S360BuildOutput,String>{
    if p.target!="s360"{return Err(format!("need 's360', got '{}'",p.target))}
    let bin=mbuild(p,false)?; std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(S360BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
impl ZArchBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<ZArchBuildOutput,String>{
    if p.target!="zarch"{return Err(format!("need 'zarch', got '{}'",p.target))}
    let bin=mbuild(p,true)?; std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(ZArchBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
