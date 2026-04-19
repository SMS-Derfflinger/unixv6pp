#ifndef MACHINE_H
#define	MACHINE_H

#include "PageDirectory.h"
#include "sys/types.h"

/*
 * Machine�����ڷ�װ�Եײ�Ӳ��������ģʽ�����ݽṹ�ĳ���
 * ������8254ʱ��оƬ��8259A�жϿ���оƬ�ĳ�ʼ�����Լ���
 * ����ģʽ��GDT, IDT�����ݽṹ�Ĳ�����
 * 
 * Machine��ʹ��Singletonģʽʵ�֣���ϵͳ�ں�������������
 * ��ֻ��һ��ʵ������
 */
class Machine
{
	/* static const member */
public:
	/* �ں˴���Ρ��ں����ݶΣ��û�����Ρ��û����ݶΣ�TSS�ε�ѡ���� */
	static const unsigned int KERNEL_CODE_SEGMENT_SELECTOR = 0x08;
	static const unsigned int KERNEL_DATA_SEGMENT_SELECTOR = 0x10;
	static const unsigned int USER_CODE_SEGMENT_SELECTOR = (0x18 | 0x3);
	static const unsigned int USER_DATA_SEGMENT_SELECTOR = (0x20 | 0x3);		
	static const unsigned int TASK_STATE_SEGMENT_SELECTOR = 0x28;
	static const unsigned int TASK_STATE_SEGMENT_IDX = 0x5;	/* TSS����������GDT�е�λ�� */

	/* ҳĿ¼������̬ҳ�����û�̬ҳ���������ڴ��е���ʼ��ַ */
	static const unsigned long PAGE_DIRECTORY_BASE_ADDRESS = 0x200000;
	static const unsigned long KERNEL_PAGE_TABLE_BASE_ADDRESS = 0x201000;

	static const unsigned long VESA_PAGE_TABLE_BASE_ADDR = 32 * 1024 * 1024;  // place it over 32M

	static const unsigned long USER_PAGE_TABLE_BASE_ADDRESS = 0x202000;
	static const unsigned long USER_PAGE_TABLE_CNT = 2;
	
	/* �ں˿ռ��С 4M 0xC0000000 - 0xC0400000 1 PageTable */
	static const unsigned int KERNEL_SPACE_SIZE = 0x400000;
	static const unsigned long KERNEL_SPACE_START_ADDRESS	= 0xC0000000;
	
public:
	static Machine& Instance();			/* ���ص�̬���instance */
	void LoadIDT();						/* �ѽ����õ�IDT���Ļ���ַ�ͳ��ȼ��ؽ�IDTR�Ĵ��� */
	void LoadGDT();						/* �ѽ����õ�GDT���Ļ���ַ�ͳ��ȼ��ؽ�IDTR�Ĵ��� */
	void LoadTaskRegister();

	void InitIDT();
	void InitGDT();

	/**
	 * VESA Support
	 *   added by 2051565 GTY
	 */
	void InitVESAMemoryMap(uintptr_t videoMemAddr, uintptr_t virtualMemAddr, size_t videoMemSize);

	void InitPageDirectory();
	void InitUserPageTable();
	void EnablePageProtection();
	
	/* property functions */
public:
	PageDirectory& GetPageDirectory();	/* ��ȡ��ǰ����ʹ�õ�ҳĿ¼�� */
	PageTable& GetKernelPageTable();	/* ��ȡ����ϵͳ�ں���ʹ�õ�ҳ��������map 0xc0000000���ϵ�ַ */
	PageTable* GetUserPageTableArray();	/* ��ȡ�û�����ҳ���������ţ���ӳ����0x202000��0x203000�ϣ�
										    ӳ��0x00000000 - 0x00800000�û�̬��ַ�ռ� */
	
private:
	static Machine instance;	/* Machine������ʵ�� */
	
	PageDirectory* m_PageDirectory;	
	PageTable*	m_KernelPageTable;
	PageTable*	m_UserPageTable;
};

#endif
