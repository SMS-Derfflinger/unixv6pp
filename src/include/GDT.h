#ifndef GDT_H
#define GDT_H

struct GDTR
{
private:
    unsigned char m_Raw[6];
}__attribute__((packed));

extern "C" void _gdt_init();
extern "C" void _gdt_form_gdtr(GDTR* gdtr);

class GDT
{
public:
    void Initialize() {
        _gdt_init();
    }

    void FormGDTR(GDTR& gdtr) {
        _gdt_form_gdtr(&gdtr);
    }
};

#endif
