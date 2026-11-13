//! Cycle-stepped Ricoh 2A03 execution engine.
//! One call to clock performs one externally visible CPU bus access.
//! This module implements regular read/write addressing sequences and branches.
//! Complex stack/control and read-modify-write sequences return explicit faults
//! until their exact bus-cycle microprograms are added.
use crate::{decode, flags, AddressingMode, CpuBus, Mnemonic, Opcode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusAccess { Read { address: u16, value: u8 }, Write { address: u16, value: u8 } }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuFault { UnsupportedInstruction { opcode: u8 }, InterruptSequencePending, Jammed { opcode: u8 } }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind { Read, Write, ReadModifyWrite, Implied, Branch, Control }

fn kind(op: &Opcode) -> Kind {
    use Mnemonic::*;
    if op.mode == AddressingMode::Accumulator {
        return Kind::Implied;
    }
    match op.mnemonic {
        ASL | LSR | ROL | ROR | INC | DEC | SLO | RLA | SRE | RRA | DCP | ISC => Kind::ReadModifyWrite,
        STA | STX | STY | SAX => Kind::Write,
        BCC | BCS | BEQ | BMI | BNE | BPL | BVC | BVS => Kind::Branch,
        BRK | JMP | JSR | RTI | RTS | PHA | PHP | PLA | PLP => Kind::Control,
        CLC | CLD | CLI | CLV | DEX | DEY | INX | INY | SEC | SED | SEI |
        TAX | TAY | TSX | TXA | TXS | TYA => Kind::Implied,
        _ => match op.mode { AddressingMode::Implied | AddressingMode::Accumulator => Kind::Implied, _ => Kind::Read }
    }
}

/// All in-flight values are explicit so a snapshot can resume between bus cycles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CycleCpuState {
    pub a:u8, pub x:u8, pub y:u8, pub sp:u8, pub pc:u16, pub p:u8,
    pub opcode:u8, pub phase:u8, pub cycles:u64, pub operand:u8,
    pub low:u8, pub base:u16, pub effective:u16, pub crossed_page:bool,
    pub halted:bool, pub nmi_pending:bool, pub irq_line:bool,
}
impl Default for CycleCpuState {
    fn default()->Self { Self { a:0,x:0,y:0,sp:0xFD,pc:0,p:flags::INTERRUPT_DISABLE|flags::UNUSED,
        opcode:0,phase:0,cycles:0,operand:0,low:0,base:0,effective:0,crossed_page:false,
        halted:false,nmi_pending:false,irq_line:false } }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CycleCpu { state: CycleCpuState }

impl CycleCpu {
    pub const fn new()->Self { Self { state:CycleCpuState { a:0,x:0,y:0,sp:0xFD,pc:0,
        p:flags::INTERRUPT_DISABLE|flags::UNUSED,opcode:0,phase:0,cycles:0,operand:0,
        low:0,base:0,effective:0,crossed_page:false,halted:false,nmi_pending:false,irq_line:false } } }
    pub const fn state(&self)->CycleCpuState { self.state }
    pub fn restore_state(&mut self,mut s:CycleCpuState){s.p|=flags::UNUSED;self.state=s;}
    pub fn set_pc(&mut self,v:u16){self.state.pc=v;}
    pub fn set_a(&mut self,v:u8){self.state.a=v;}
    pub fn set_x(&mut self,v:u8){self.state.x=v;}
    pub fn set_y(&mut self,v:u8){self.state.y=v;}
    pub fn set_irq_line(&mut self,v:bool){self.state.irq_line=v;}
    pub fn request_nmi(&mut self){self.state.nmi_pending=true;}

    fn read<B:CpuBus>(&mut self,b:&mut B,a:u16)->BusAccess { let v=b.read(a);BusAccess::Read{address:a,value:v} }
    fn write<B:CpuBus>(&mut self,b:&mut B,a:u16,v:u8)->BusAccess { b.write(a,v);BusAccess::Write{address:a,value:v} }
    fn fetch<B:CpuBus>(&mut self,b:&mut B)->(u8,BusAccess) { let a=self.state.pc;let v=b.read(a);self.state.pc=self.state.pc.wrapping_add(1);(v,BusAccess::Read{address:a,value:v}) }
    fn finish(&mut self){self.state.phase=0;self.state.crossed_page=false;}
    fn nz(&mut self,v:u8){self.state.p=(self.state.p&!(flags::NEGATIVE|flags::ZERO))|if v==0{flags::ZERO}else{0}|(v&flags::NEGATIVE)|flags::UNUSED;}
    fn compare(&mut self,a:u8,b:u8){let r=a.wrapping_sub(b);self.state.p=(self.state.p&!(flags::CARRY|flags::ZERO|flags::NEGATIVE))|if a>=b{flags::CARRY}else{0}|if a==b{flags::ZERO}else{0}|(r&flags::NEGATIVE)|flags::UNUSED;}
    fn adc(&mut self,v:u8){let a=self.state.a;let sum=u16::from(a)+u16::from(v)+if self.state.p&flags::CARRY!=0 { 1 } else { 0 };let r=sum as u8;
        self.state.p&=!(flags::CARRY|flags::OVERFLOW);if sum>255{self.state.p|=flags::CARRY;}
        if (!(a^v)&(a^r)&0x80)!=0{self.state.p|=flags::OVERFLOW;}self.state.a=r;self.nz(r);}
    fn apply(&mut self,m:Mnemonic,v:u8)->Result<(),CpuFault>{use Mnemonic::*;match m{
        ADC=>self.adc(v), SBC=>self.adc(!v), AND=>{self.state.a&=v;self.nz(self.state.a)},
        ORA=>{self.state.a|=v;self.nz(self.state.a)}, EOR=>{self.state.a^=v;self.nz(self.state.a)},
        LDA=>{self.state.a=v;self.nz(v)},LDX=>{self.state.x=v;self.nz(v)},LDY=>{self.state.y=v;self.nz(v)},
        LAX=>{self.state.a=v;self.state.x=v;self.nz(v)},CMP=>self.compare(self.state.a,v),
        CPX=>self.compare(self.state.x,v),CPY=>self.compare(self.state.y,v),
        BIT=>{self.state.p=(self.state.p&!(flags::ZERO|flags::NEGATIVE|flags::OVERFLOW))|if self.state.a&v==0{flags::ZERO}else{0}|(v&(flags::NEGATIVE|flags::OVERFLOW))|flags::UNUSED;},
        NOP=>{},_=>return Err(CpuFault::UnsupportedInstruction{opcode:self.state.opcode})}Ok(())}
    fn store_value(&self,m:Mnemonic)->Result<u8,CpuFault>{match m{Mnemonic::STA=>Ok(self.state.a),Mnemonic::STX=>Ok(self.state.x),Mnemonic::STY=>Ok(self.state.y),Mnemonic::SAX=>Ok(self.state.a&self.state.x),_=>Err(CpuFault::UnsupportedInstruction{opcode:self.state.opcode})}}
    fn branch(&self,m:Mnemonic)->bool{use Mnemonic::*;match m{
        BCC=>self.state.p&flags::CARRY==0,BCS=>self.state.p&flags::CARRY!=0,
        BEQ=>self.state.p&flags::ZERO!=0,BNE=>self.state.p&flags::ZERO==0,
        BMI=>self.state.p&flags::NEGATIVE!=0,BPL=>self.state.p&flags::NEGATIVE==0,
        BVC=>self.state.p&flags::OVERFLOW==0,BVS=>self.state.p&flags::OVERFLOW!=0,_=>false}}
    fn transform(&mut self, m: Mnemonic, value: u8) -> Result<u8, CpuFault> {
        use Mnemonic::*;
        let carry_in = u8::from(self.state.p & flags::CARRY != 0);
        let result = match m {
            ASL | SLO => {
                self.state.p = (self.state.p & !flags::CARRY) | if value & 0x80 != 0 { flags::CARRY } else { 0 };
                value.wrapping_shl(1)
            }
            LSR | SRE => {
                self.state.p = (self.state.p & !flags::CARRY) | (value & 1);
                value >> 1
            }
            ROL | RLA => {
                self.state.p = (self.state.p & !flags::CARRY) | if value & 0x80 != 0 { flags::CARRY } else { 0 };
                (value << 1) | carry_in
            }
            ROR | RRA => {
                self.state.p = (self.state.p & !flags::CARRY) | (value & 1);
                (value >> 1) | (carry_in << 7)
            }
            INC | ISC => value.wrapping_add(1),
            DEC | DCP => value.wrapping_sub(1),
            _ => return Err(CpuFault::UnsupportedInstruction { opcode: self.state.opcode }),
        };
        Ok(result)
    }

    fn finish_rmw(&mut self, m: Mnemonic, value: u8) -> Result<(), CpuFault> {
        use Mnemonic::*;
        match m {
            SLO => { self.state.a |= value; self.nz(self.state.a); }
            RLA => { self.state.a &= value; self.nz(self.state.a); }
            SRE => { self.state.a ^= value; self.nz(self.state.a); }
            RRA => self.adc(value),
            DCP => self.compare(self.state.a, value),
            ISC => self.adc(!value),
            _ => self.nz(value),
        }
        Ok(())
    }

    fn rmw_cycle<B: CpuBus>(&mut self, b: &mut B, op: Opcode, ph: u8) -> Result<BusAccess, CpuFault> {
        let mode = op.mode;
        let access = match mode {
            AddressingMode::ZeroPage => match ph {
                1 => { let (v, a) = self.fetch(b); self.state.effective = u16::from(v); self.state.phase = 2; a }
                2 => { let a = self.read(b, self.state.effective); if let BusAccess::Read { value, .. } = a { self.state.operand = value; } self.state.phase = 3; a }
                3 => { let new = self.transform(op.mnemonic, self.state.operand)?; self.state.low = new; self.state.phase = 4; self.write(b, self.state.effective, self.state.operand) }
                _ => { let new = self.state.low; let a = self.write(b, self.state.effective, new); self.finish_rmw(op.mnemonic, new)?; self.finish(); a }
            },
            AddressingMode::ZeroPageX => match ph {
                1 => { let (v, a) = self.fetch(b); self.state.operand = v; self.state.phase = 2; a }
                2 => { let a = self.read(b, u16::from(self.state.operand)); self.state.effective = u16::from(self.state.operand.wrapping_add(self.state.x)); self.state.phase = 3; a }
                3 => { let a = self.read(b, self.state.effective); if let BusAccess::Read { value, .. } = a { self.state.operand = value; } self.state.phase = 4; a }
                4 => { self.state.low = self.transform(op.mnemonic, self.state.operand)?; self.state.phase = 5; self.write(b, self.state.effective, self.state.operand) }
                _ => { let new = self.state.low; let a = self.write(b, self.state.effective, new); self.finish_rmw(op.mnemonic, new)?; self.finish(); a }
            },
            AddressingMode::Absolute => match ph {
                1 => { let (v, a) = self.fetch(b); self.state.low = v; self.state.phase = 2; a }
                2 => { let (v, a) = self.fetch(b); self.state.effective = u16::from_le_bytes([self.state.low, v]); self.state.phase = 3; a }
                3 => { let a = self.read(b, self.state.effective); if let BusAccess::Read { value, .. } = a { self.state.operand = value; } self.state.phase = 4; a }
                4 => { self.state.low = self.transform(op.mnemonic, self.state.operand)?; self.state.phase = 5; self.write(b, self.state.effective, self.state.operand) }
                _ => { let new = self.state.low; let a = self.write(b, self.state.effective, new); self.finish_rmw(op.mnemonic, new)?; self.finish(); a }
            },
            AddressingMode::AbsoluteX => match ph {
                1 => { let (v, a) = self.fetch(b); self.state.low = v; self.state.phase = 2; a }
                2 => { let (v, a) = self.fetch(b); self.state.base = u16::from_le_bytes([self.state.low, v]); self.state.effective = self.state.base.wrapping_add(u16::from(self.state.x)); self.state.phase = 3; a }
                3 => { let addr = (self.state.base & 0xFF00) | (self.state.effective & 0x00FF); self.state.phase = 4; self.read(b, addr) }
                4 => { let a = self.read(b, self.state.effective); if let BusAccess::Read { value, .. } = a { self.state.operand = value; } self.state.phase = 5; a }
                5 => { self.state.low = self.transform(op.mnemonic, self.state.operand)?; self.state.phase = 6; self.write(b, self.state.effective, self.state.operand) }
                _ => { let new = self.state.low; let a = self.write(b, self.state.effective, new); self.finish_rmw(op.mnemonic, new)?; self.finish(); a }
            },
            AddressingMode::IndexedIndirect => match ph {
                1 => { let (v, a) = self.fetch(b); self.state.operand = v; self.state.phase = 2; a }
                2 => { let a = self.read(b, u16::from(self.state.operand)); self.state.operand = self.state.operand.wrapping_add(self.state.x); self.state.phase = 3; a }
                3 => { let addr = u16::from(self.state.operand); let v = b.read(addr); self.state.low = v; self.state.phase = 4; BusAccess::Read { address: addr, value: v } }
                4 => { let addr = u16::from(self.state.operand.wrapping_add(1)); let v = b.read(addr); self.state.effective = u16::from_le_bytes([self.state.low, v]); self.state.phase = 5; BusAccess::Read { address: addr, value: v } }
                5 => { let a = self.read(b, self.state.effective); if let BusAccess::Read { value, .. } = a { self.state.operand = value; } self.state.phase = 6; a }
                6 => { self.state.low = self.transform(op.mnemonic, self.state.operand)?; self.state.phase = 7; self.write(b, self.state.effective, self.state.operand) }
                _ => { let new = self.state.low; let a = self.write(b, self.state.effective, new); self.finish_rmw(op.mnemonic, new)?; self.finish(); a }
            },
            AddressingMode::IndirectIndexed => match ph {
                1 => { let (v, a) = self.fetch(b); self.state.operand = v; self.state.phase = 2; a }
                2 => { let addr = u16::from(self.state.operand); let v = b.read(addr); self.state.low = v; self.state.phase = 3; BusAccess::Read { address: addr, value: v } }
                3 => { let addr = u16::from(self.state.operand.wrapping_add(1)); let v = b.read(addr); self.state.base = u16::from_le_bytes([self.state.low, v]); self.state.effective = self.state.base.wrapping_add(u16::from(self.state.y)); self.state.phase = 4; BusAccess::Read { address: addr, value: v } }
                4 => { let addr = (self.state.base & 0xFF00) | (self.state.effective & 0x00FF); self.state.phase = 5; self.read(b, addr) }
                5 => { let a = self.read(b, self.state.effective); if let BusAccess::Read { value, .. } = a { self.state.operand = value; } self.state.phase = 6; a }
                6 => { self.state.low = self.transform(op.mnemonic, self.state.operand)?; self.state.phase = 7; self.write(b, self.state.effective, self.state.operand) }
                _ => { let new = self.state.low; let a = self.write(b, self.state.effective, new); self.finish_rmw(op.mnemonic, new)?; self.finish(); a }
            },
            _ => return Err(CpuFault::UnsupportedInstruction { opcode: self.state.opcode }),
        };
        Ok(access)
    }

    fn implied(&mut self,m:Mnemonic)->Result<(),CpuFault>{use Mnemonic::*;match m{
        CLC=>self.state.p&=!flags::CARRY,CLD=>self.state.p&=!flags::DECIMAL,CLI=>self.state.p&=!flags::INTERRUPT_DISABLE,
        CLV=>self.state.p&=!flags::OVERFLOW,SEC=>self.state.p|=flags::CARRY,SED=>self.state.p|=flags::DECIMAL,SEI=>self.state.p|=flags::INTERRUPT_DISABLE,
        DEX=>{self.state.x=self.state.x.wrapping_sub(1);self.nz(self.state.x)},DEY=>{self.state.y=self.state.y.wrapping_sub(1);self.nz(self.state.y)},
        INX=>{self.state.x=self.state.x.wrapping_add(1);self.nz(self.state.x)},INY=>{self.state.y=self.state.y.wrapping_add(1);self.nz(self.state.y)},
        TAX=>{self.state.x=self.state.a;self.nz(self.state.x)},TAY=>{self.state.y=self.state.a;self.nz(self.state.y)},
        TSX=>{self.state.x=self.state.sp;self.nz(self.state.x)},TXA=>{self.state.a=self.state.x;self.nz(self.state.a)},
        TXS=>self.state.sp=self.state.x,TYA=>{self.state.a=self.state.y;self.nz(self.state.a)},
        ASL=>{let value=self.state.a;let result=self.transform(ASL,value)?;self.state.a=result;self.nz(result)},
        LSR=>{let value=self.state.a;let result=self.transform(LSR,value)?;self.state.a=result;self.nz(result)},
        ROL=>{let value=self.state.a;let result=self.transform(ROL,value)?;self.state.a=result;self.nz(result)},
        ROR=>{let value=self.state.a;let result=self.transform(ROR,value)?;self.state.a=result;self.nz(result)},
        NOP=>{},
        _=>return Err(CpuFault::UnsupportedInstruction{opcode:self.state.opcode})}self.state.p|=flags::UNUSED;Ok(())}

    /// Executes one bus cycle. It never performs more than one bus read/write.
    pub fn clock<B:CpuBus>(&mut self,b:&mut B)->Result<BusAccess,CpuFault>{
        if self.state.halted{return Err(CpuFault::Jammed{opcode:self.state.opcode});}
        self.state.cycles=self.state.cycles.wrapping_add(1);
        if self.state.phase==0 {
            let (v,a)=self.fetch(b);self.state.opcode=v;self.state.phase=1;
            if decode(v).jams_cpu{self.state.halted=true;}return Ok(a);
        }
        let op=*decode(self.state.opcode);let ph=self.state.phase;
        match kind(&op){
            Kind::Implied=>{let (_,a)=self.fetch(b);self.implied(op.mnemonic)?;self.finish();Ok(a)}
            Kind::Branch=>{
                if ph==1{let(v,a)=self.fetch(b);self.state.operand=v;if !self.branch(op.mnemonic){self.finish();}
                    else{self.state.base=self.state.pc;self.state.effective=self.state.pc.wrapping_add((v as i8 as i16) as u16);
                        self.state.crossed_page=self.state.pc&0xFF00!=self.state.effective&0xFF00;self.state.phase=2;}Ok(a)}
                else{let old=self.state.pc;let addr=if ph==2{old}else{(old&0xFF00)|(self.state.effective&0xFF)};
                    let a=self.read(b,addr);if ph==2&&self.state.crossed_page{self.state.pc=(old&0xFF00)|(self.state.effective&0xFF);self.state.phase=3;}
                    else{self.state.pc=self.state.effective;self.finish();}Ok(a)}
            }
            Kind::Control=>Err(CpuFault::UnsupportedInstruction{opcode:self.state.opcode}),
            Kind::ReadModifyWrite=>self.rmw_cycle(b,op,ph),
            Kind::Read|Kind::Write=>self.memory_cycle(b,op,ph)
        }
    }

    fn memory_cycle<B:CpuBus>(&mut self,b:&mut B,op:Opcode,ph:u8)->Result<BusAccess,CpuFault>{
        let wr=kind(&op)==Kind::Write;let idx=match op.mode{AddressingMode::ZeroPageX|AddressingMode::AbsoluteX=>self.state.x,
            AddressingMode::ZeroPageY|AddressingMode::AbsoluteY=>self.state.y,_=>0};
        let a=match op.mode{
            AddressingMode::Immediate=>{let(v,a)=self.fetch(b);self.apply(op.mnemonic,v)?;self.finish();a}
            AddressingMode::ZeroPage=>if ph==1{let(v,a)=self.fetch(b);self.state.effective=u16::from(v);self.state.phase=2;a}else{
                let a=if wr{self.write(b,self.state.effective,self.store_value(op.mnemonic)?)}else{let a=self.read(b,self.state.effective);if let BusAccess::Read{value,..}=a{self.apply(op.mnemonic,value)?;}a};self.finish();a}
            AddressingMode::ZeroPageX|AddressingMode::ZeroPageY=>match ph{
                1=>{let(v,a)=self.fetch(b);self.state.operand=v;self.state.phase=2;a}
                2=>{let a=self.read(b,u16::from(self.state.operand));self.state.effective=u16::from(self.state.operand.wrapping_add(idx));self.state.phase=3;a}
                _=>{let a=if wr{self.write(b,self.state.effective,self.store_value(op.mnemonic)?)}else{let a=self.read(b,self.state.effective);if let BusAccess::Read{value,..}=a{self.apply(op.mnemonic,value)?;}a};self.finish();a}}
            AddressingMode::Absolute=>match ph{
                1=>{let(v,a)=self.fetch(b);self.state.low=v;self.state.phase=2;a}
                2=>{let(v,a)=self.fetch(b);self.state.effective=u16::from_le_bytes([self.state.low,v]);self.state.phase=3;a}
                _=>{let a=if wr{self.write(b,self.state.effective,self.store_value(op.mnemonic)?)}else{let a=self.read(b,self.state.effective);if let BusAccess::Read{value,..}=a{self.apply(op.mnemonic,value)?;}a};self.finish();a}}
            AddressingMode::AbsoluteX|AddressingMode::AbsoluteY=>match ph{
                1=>{let(v,a)=self.fetch(b);self.state.low=v;self.state.phase=2;a}
                2=>{let(v,a)=self.fetch(b);self.state.base=u16::from_le_bytes([self.state.low,v]);self.state.effective=self.state.base.wrapping_add(u16::from(idx));
                    self.state.crossed_page=self.state.base&0xFF00!=self.state.effective&0xFF00;self.state.phase=3;a}
                3=>{let addr=(self.state.base&0xFF00)|(self.state.effective&0xFF);let a=self.read(b,addr);
                    if wr||self.state.crossed_page{self.state.phase=4;}else{if let BusAccess::Read{value,..}=a{self.apply(op.mnemonic,value)?;}self.finish();}a}
                _=>{let a=if wr{self.write(b,self.state.effective,self.store_value(op.mnemonic)?)}else{let a=self.read(b,self.state.effective);if let BusAccess::Read{value,..}=a{self.apply(op.mnemonic,value)?;}a};self.finish();a}}
            AddressingMode::IndexedIndirect=>match ph{
                1=>{let(v,a)=self.fetch(b);self.state.operand=v;self.state.phase=2;a}
                2=>{let a=self.read(b,u16::from(self.state.operand));self.state.operand=self.state.operand.wrapping_add(self.state.x);self.state.phase=3;a}
                3=>{let addr=u16::from(self.state.operand);let v=b.read(addr);self.state.low=v;self.state.phase=4;BusAccess::Read{address:addr,value:v}}
                4=>{let addr=u16::from(self.state.operand.wrapping_add(1));let v=b.read(addr);self.state.effective=u16::from_le_bytes([self.state.low,v]);self.state.phase=5;BusAccess::Read{address:addr,value:v}}
                _=>{let a=if wr{self.write(b,self.state.effective,self.store_value(op.mnemonic)?)}else{let a=self.read(b,self.state.effective);if let BusAccess::Read{value,..}=a{self.apply(op.mnemonic,value)?;}a};self.finish();a}}
            AddressingMode::IndirectIndexed=>match ph{
                1=>{let(v,a)=self.fetch(b);self.state.operand=v;self.state.phase=2;a}
                2=>{let addr=u16::from(self.state.operand);let v=b.read(addr);self.state.low=v;self.state.phase=3;BusAccess::Read{address:addr,value:v}}
                3=>{let addr=u16::from(self.state.operand.wrapping_add(1));let v=b.read(addr);self.state.base=u16::from_le_bytes([self.state.low,v]);
                    self.state.effective=self.state.base.wrapping_add(u16::from(self.state.y));self.state.crossed_page=self.state.base&0xFF00!=self.state.effective&0xFF00;
                    self.state.phase=4;BusAccess::Read{address:addr,value:v}}
                4=>{let addr=(self.state.base&0xFF00)|(self.state.effective&0xFF);let a=self.read(b,addr);
                    if wr||self.state.crossed_page{self.state.phase=5;}else{if let BusAccess::Read{value,..}=a{self.apply(op.mnemonic,value)?;}self.finish();}a}
                _=>{let a=if wr{self.write(b,self.state.effective,self.store_value(op.mnemonic)?)}else{let a=self.read(b,self.state.effective);if let BusAccess::Read{value,..}=a{self.apply(op.mnemonic,value)?;}a};self.finish();a}}
            _=>return Err(CpuFault::UnsupportedInstruction{opcode:self.state.opcode})
        };Ok(a)
    }
}

#[cfg(test)]
mod tests{
 use super::*;use std::collections::BTreeMap;
 #[derive(Default)]struct Ram{m:BTreeMap<u16,u8>,log:Vec<BusAccess>}
 impl CpuBus for Ram{fn read(&mut self,a:u16)->u8{let v=*self.m.get(&a).unwrap_or(&0);self.log.push(BusAccess::Read{address:a,value:v});v}
 fn write(&mut self,a:u16,v:u8){self.m.insert(a,v);self.log.push(BusAccess::Write{address:a,value:v});}}
 #[test]fn lda_immediate_is_two_bus_cycles(){let mut b=Ram::default();b.m.insert(0x8000,0xA9);b.m.insert(0x8001,0x42);let mut c=CycleCpu::new();c.set_pc(0x8000);
 c.clock(&mut b).unwrap();assert_eq!(c.state().phase,1);c.clock(&mut b).unwrap();assert_eq!(c.state().a,0x42);assert_eq!(c.state().cycles,2);assert_eq!(c.state().phase,0);}
 #[test]fn zero_page_sta_writes_on_third_cycle(){let mut b=Ram::default();b.m.insert(0x8000,0x85);b.m.insert(0x8001,0x10);let mut c=CycleCpu::new();c.set_pc(0x8000);c.set_a(0x77);
 c.clock(&mut b).unwrap();c.clock(&mut b).unwrap();assert_eq!(c.clock(&mut b).unwrap(),BusAccess::Write{address:0x10,value:0x77});}
 #[test]fn absolute_indexed_read_has_page_cross_cycle(){let mut b=Ram::default();b.m.insert(0x8000,0xBD);b.m.insert(0x8001,0xFF);b.m.insert(0x8002,0x20);b.m.insert(0x2100,0x66);
 let mut c=CycleCpu::new();c.set_pc(0x8000);c.set_x(1);for _ in 0..5{c.clock(&mut b).unwrap();}assert_eq!(c.state().a,0x66);assert_eq!(c.state().cycles,5);}
 #[test]fn snapshot_resumes_mid_instruction(){let mut b=Ram::default();b.m.insert(0x9000,0xA9);b.m.insert(0x9001,0x31);let mut c=CycleCpu::new();c.set_pc(0x9000);c.clock(&mut b).unwrap();
 let mut resumed=CycleCpu::new();resumed.restore_state(c.state());resumed.clock(&mut b).unwrap();assert_eq!(resumed.state().a,0x31);assert_eq!(resumed.state().phase,0);}

 #[test]
 fn zero_page_asl_performs_read_old_write_and_write_new() {
     let mut b=Ram::default();
     b.m.insert(0x8000,0x06); // ASL $10
     b.m.insert(0x8001,0x10);
     b.m.insert(0x0010,0x81);
     let mut c=CycleCpu::new(); c.set_pc(0x8000);
     let accesses: Vec<_>=(0..5).map(|_|c.clock(&mut b).unwrap()).collect();
     assert_eq!(accesses, vec![
         BusAccess::Read{address:0x8000,value:0x06},
         BusAccess::Read{address:0x8001,value:0x10},
         BusAccess::Read{address:0x0010,value:0x81},
         BusAccess::Write{address:0x0010,value:0x81},
         BusAccess::Write{address:0x0010,value:0x02},
     ]);
     assert_eq!(b.m.get(&0x0010),Some(&0x02));
     assert_ne!(c.state().p & flags::CARRY,0);
     assert_eq!(c.state().phase,0);
 }

 #[test]
 fn absolute_x_rmw_has_index_dummy_read_and_two_writes() {
     let mut b=Ram::default();
     b.m.insert(0x8000,0x1E); // ASL $20FF,X
     b.m.insert(0x8001,0xFF);
     b.m.insert(0x8002,0x20);
     b.m.insert(0x2000,0x00);
     b.m.insert(0x2100,0x40);
     let mut c=CycleCpu::new(); c.set_pc(0x8000); c.set_x(1);
     let accesses: Vec<_>=(0..7).map(|_|c.clock(&mut b).unwrap()).collect();
     assert_eq!(accesses[3],BusAccess::Read{address:0x2000,value:0x00});
     assert_eq!(accesses[4],BusAccess::Read{address:0x2100,value:0x40});
     assert_eq!(accesses[5],BusAccess::Write{address:0x2100,value:0x40});
     assert_eq!(accesses[6],BusAccess::Write{address:0x2100,value:0x80});
 }

 #[test]
 fn accumulator_lsr_updates_carry_and_zero_flags() {
     let mut b=Ram::default();
     b.m.insert(0x8000,0x4A);
     let mut c=CycleCpu::new(); c.set_pc(0x8000); c.set_a(1);
     c.clock(&mut b).unwrap(); c.clock(&mut b).unwrap();
     assert_eq!(c.state().a,0);
     assert_ne!(c.state().p & flags::CARRY,0);
     assert_ne!(c.state().p & flags::ZERO,0);
 }


 #[test]
 fn unofficial_slo_indexed_indirect_uses_eight_cycles() {
     let mut b=Ram::default();
     b.m.insert(0x8000,0x03); // SLO ($20,X)
     b.m.insert(0x8001,0x20);
     b.m.insert(0x0021,0x00);
     b.m.insert(0x0022,0x30);
     b.m.insert(0x3000,0x81);
     let mut c=CycleCpu::new(); c.set_pc(0x8000); c.set_x(1); c.set_a(0x01);
     let accesses: Vec<_>=(0..8).map(|_|c.clock(&mut b).unwrap()).collect();
     assert_eq!(accesses[5],BusAccess::Read{address:0x3000,value:0x81});
     assert_eq!(accesses[6],BusAccess::Write{address:0x3000,value:0x81});
     assert_eq!(accesses[7],BusAccess::Write{address:0x3000,value:0x02});
     assert_eq!(c.state().a,0x03);
 }

 #[test]
 fn unofficial_dcp_indirect_indexed_uses_eight_cycles() {
     let mut b=Ram::default();
     b.m.insert(0x8000,0xD3); // DCP ($20),Y
     b.m.insert(0x8001,0x20);
     b.m.insert(0x0020,0xFF);
     b.m.insert(0x0021,0x20);
     b.m.insert(0x2100,0x10);
     let mut c=CycleCpu::new(); c.set_pc(0x8000); c.set_y(1); c.set_a(0x0F);
     let accesses: Vec<_>=(0..8).map(|_|c.clock(&mut b).unwrap()).collect();
     assert_eq!(accesses[4],BusAccess::Read{address:0x2000,value:0x00});
     assert_eq!(accesses[5],BusAccess::Read{address:0x2100,value:0x10});
     assert_eq!(accesses[6],BusAccess::Write{address:0x2100,value:0x10});
     assert_eq!(accesses[7],BusAccess::Write{address:0x2100,value:0x0F});
     assert_eq!(c.state().p & flags::CARRY,0);
 }

}
