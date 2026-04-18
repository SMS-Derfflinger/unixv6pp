#ifndef IDT_H
#define IDT_H

struct IDTR
{
private:
    unsigned char m_Raw[6];
}__attribute__((packed));

extern "C" void _idt_init();
extern "C" void _idt_default_interrupt_handler();
extern "C" void _idt_default_exception_handler();
extern "C" void _idt_set_interrupt_gate(int number, unsigned int handler);
extern "C" void _idt_set_trap_gate(int number, unsigned int handler);
extern "C" void _idt_form_idtr(IDTR* idtr);

class IDT
{
public:
    void Initialize() {
        _idt_init();
    }

    static void DefaultInterruptHandler() {
        _idt_default_interrupt_handler();
    }

    static void DefaultExceptionHandler() {
        _idt_default_exception_handler();
    }

    void SetInterruptGate(int number, unsigned int handler) {
        _idt_set_interrupt_gate(number, handler);
    }

    void SetTrapGate(int number, unsigned int handler) {
        _idt_set_trap_gate(number, handler);
    }

    void FormIDTR(IDTR& idtr) {
        _idt_form_idtr(&idtr);
    }
};

#endif
