#include "Process.h"
#include "ProcessManager.h"
#include "Kernel.h"
#include "Utility.h"
#include "Machine.h"
#include "Video.h"

extern "C" {
	void Process_send_signal(Process*);
	void Process_process_signal(Process*, struct pt_context*);
	void Process_set_nice(Process*);
	bool Process_should_process(Process*);
	void Process_raise(Process*, int);
}

Process::Process()
{
	/* ��ʶ����p_statΪSNULL����ʶ�ý��������ʹ�� */
	this->p_stat = SNULL;
	/* ����0#������Wait()ʱ���������process����0#����Ϊ������ */
	this->p_ppid = -1;
}

Process::~Process()
{
}


void Process::SetRun()
{
	ProcessManager& procMgr = Kernel::Instance().GetProcessManager();

	/* ���˯��ԭ��תΪ����״̬ */
	this->p_wchan = 0;
	this->p_stat = Process::SRUN;
	if ( this->p_pri < procMgr.CurPri )
	{
		procMgr.RunRun++;
	}
	if ( 0 != procMgr.RunOut && (this->p_flag & Process::SLOAD) == 0 )
	{
		procMgr.RunOut = 0;
		procMgr.WakeUpAll((unsigned long)&procMgr.RunOut);
	}
}

void Process::SetPri()
{
	int priority;
	ProcessManager& procMgr = Kernel::Instance().GetProcessManager();

	priority = this->p_cpu / 16;
	priority += ProcessManager::PUSER + this->p_nice;

	if ( priority > 255 )
	{
		priority = 255;
	}
	if ( priority > procMgr.CurPri )
	{
		procMgr.RunRun++;
	}
	this->p_pri = priority;
}

bool Process::IsSleepOn(unsigned long chan)
{
	/* ��鵱ǰ����˯��ԭ���Ƿ�Ϊchan */
	if( this->p_wchan == chan
		&& (this->p_stat == Process::SWAIT || this->p_stat == Process::SSLEEP) )
	{
		return true;
	}
	return false;
}

extern "C" void _sleep(unsigned long chan, int pri) {
        User_get_procp()->Sleep(chan, pri);
}

void Process::Sleep(unsigned long chan, int pri)
{
	User& u = Kernel::Instance().GetUser();
	ProcessManager& procMgr = Kernel::Instance().GetProcessManager();

	if ( pri > 0 )
	{
		/*
		 * �����ڽ��������Ȩ˯��֮ǰ���Լ�������֮��������յ����ɺ���
		 * ���źţ���ִֹͣ��Sleep()��ͨ��aRetU()ֱ����ת��Trap1()����
		 */
		if ( this->IsSig() )
		{
			/* returnȷ��aRetU()���ص�SystemCall::Trap1()֮������ִ��ret����ָ�� */
			aRetU(User_get_qsav());
			return;
		}
		/*
		* �˴����жϽ����ٽ�������֤����������˯��ԭ��chan��
		* �Ľ���״̬ΪSSLEEP֮�䲻�ᷢ���л���
		*/
		X86Assembly::CLI();
		this->p_wchan = chan;
		/* ����˯�����ȼ�priȷ�����̽���ߡ�������Ȩ˯�� */
		this->p_stat = Process::SWAIT;
		this->p_pri = pri;
		X86Assembly::STI();

		if ( procMgr.RunIn != 0 )
		{
			procMgr.RunIn = 0;
			procMgr.WakeUpAll((unsigned long)&procMgr.RunIn);
		}
		/* ��ǰ���̷���CPU���л�����������̨ */
		//Diagnose::Write("Process %d Start Sleep!\n", this->p_pid);
		Kernel::Instance().GetProcessManager().Swtch();
		//Diagnose::Write("Process %d End Sleep!\n", this->p_pid);
		/* ������֮���ٴμ���ź� */
		if ( this->IsSig() )
		{
			/* returnȷ��aRetU()���ص�SystemCall::Trap1()֮������ִ��ret����ָ�� */
			aRetU(User_get_qsav());
			return;
		}
	}
	else
	{
		X86Assembly::CLI();
		this->p_wchan = chan;
		/* ����˯�����ȼ�priȷ�����̽���ߡ�������Ȩ˯�� */
		this->p_stat = Process::SSLEEP;
		this->p_pri = pri;
		X86Assembly::STI();

		/* ��ǰ���̷���CPU���л�����������̨ */
		//Diagnose::Write("Process %d Start Sleep!\n", this->p_pid);
		Kernel::Instance().GetProcessManager().Swtch();
		//Diagnose::Write("Process %d End Sleep!\n", this->p_pid);
	}
}

void Process::Expand(unsigned int newSize)
{
	UserPageManager& userPgMgr = Kernel::Instance().GetUserPageManager();
	ProcessManager& procMgr = Kernel::Instance().GetProcessManager();
	User& u = Kernel::Instance().GetUser();
	Process* pProcess = User_get_procp();

	unsigned int oldSize = pProcess->p_size;
	p_size = newSize;
	unsigned long oldAddress = pProcess->p_addr;
	unsigned long newAddress;

	/* �������ͼ����С�����ͷŶ�����ڴ� */
	// if ( oldSize >= newSize )
	// {
	// 	userPgMgr.FreeMemory(oldSize - newSize, oldAddress + newSize);
	// 	return;
	// }

	/* ����ͼ��������ҪѰ��һ���СnewSize�������ڴ��� */
	SaveU(User_get_rsav());
	newAddress = userPgMgr.AllocMemory(newSize);
	/* �����ڴ�ʧ�ܣ���������ʱ�������������� */
	if ( NULL == newAddress )
	{
		SaveU(User_get_ssav());
		procMgr.XSwap(pProcess, true, oldSize);
		pProcess->p_flag |= Process::SSWAP;
		procMgr.Swtch();
		/* no return */
	}
	/* �����ڴ�ɹ���������ͼ�񿽱������ڴ�����Ȼ����ת�����ڴ����������� */
	pProcess->p_addr = newAddress;

	unsigned long copySize = oldSize;
	if (newSize < copySize)
		copySize = newSize;

	for ( unsigned int i = 0; i < copySize; i++ )
	{
		Utility::CopySeg(oldAddress + i, newAddress + i);
	}

	/* �ͷ�ԭ��ռ�õ��ڴ��� */
	userPgMgr.FreeMemory(oldSize, oldAddress);

	X86Assembly::CLI();
	SwtchUStruct(pProcess);
	RetU();
	X86Assembly::STI();

	User_get_MemoryDescriptor().MapToPageTable();
}

void Process::Exit()
{
	int i;
	User& u = Kernel::Instance().GetUser();
	ProcessManager& procMgr = Kernel::Instance().GetProcessManager();

	Diagnose::Write("Process %d is exiting\n",User_get_procp()->p_pid);
	/* Reset Tracing flag */
	User_get_procp()->p_flag &= (~Process::STRC);

	/* ������̵��źŴ�������������Ϊ1��ʾ���Ը��ź����κδ��� */
	for ( i = 0; i < User::NSIG; i++ )
	{
		User_get_signal()[i] = 1;
	}

	/* �رս��̴��ļ� */
	for ( i = 0; i < OpenFiles::NOFILES; i++ )
	{
		File* pFile = NULL;
		if ( (pFile = OpenFiles_get_file(i)) != NULL )
		{
			OpenFileTable_f_close(pFile);
			OpenFiles_set_file(i, NULL);
		}
	}
	/*  ���ʲ����ڵ�fd�����error code�����User_get_error()����Ӱ���������ִ������ */
	User_get_error() = User::NOERROR;

	/* �ݼ���ǰĿ¼�����ü��� */
	InodeTable_put(User_get_cdir());

	/* �ͷŸý��̶Թ������Ķε����� */
	if ( User_get_procp()->p_textp != NULL )
	{
		User_get_procp()->p_textp->XFree();
		User_get_procp()->p_textp = NULL;
	}

	/* ��u��д�뽻�������ȴ����������ƺ��� */
	SwapperManager& swapperMgr = Kernel::Instance().GetSwapperManager();
	BufferManager& bufMgr = Kernel::Instance().GetBufferManager();
	/* u���Ĵ�С���ᳬ��512�ֽڣ�����ֻд��ppda����ǰ512�ֽڣ�������u�ṹ��ȫ����Ϣ */
	int blkno = swapperMgr.AllocSwap(BufferManager::BUFFER_SIZE);
	if ( NULL == blkno )
	{
		Utility::Panic("Out of Swapper Space");
	}
	Buf* pBuf = bufMgr.GetBlk(DeviceManager::ROOTDEV, blkno);
	Utility::DWordCopy((int *)&u, (int *)pBuf->b_addr, BufferManager::BUFFER_SIZE / sizeof(int));
	bufMgr.Bwrite(pBuf);

	/* �ͷ��ڴ���Դ */
	User_get_MemoryDescriptor().Release();
	Process* current = User_get_procp();
	UserPageManager& userPageMgr = Kernel::Instance().GetUserPageManager();
	userPageMgr.FreeMemory(current->p_size, current->p_addr);
	current->p_addr = blkno;
	current->p_stat = Process::SZOMB;

	/* ���Ѹ����̽����ƺ��� */
	for ( i = 0; i < ProcessManager::NPROC; i++ )
	{
		if ( procMgr.process[i].p_pid == current->p_ppid )
		{
			procMgr.WakeUpAll((unsigned long)&procMgr.process[i]);
			break;
		}
	}
	/* û�ҵ������� */
	if ( ProcessManager::NPROC == i )
	{
		current->p_ppid = 1;
		procMgr.WakeUpAll((unsigned long)&procMgr.process[1]);
	}

	/* ���Լ����ӽ��̴����Լ��ĸ����� */
	for ( i = 0; i < ProcessManager::NPROC; i++ )
	{
		if ( current->p_pid == procMgr.process[i].p_ppid )
		{
			Diagnose::Write("My:%d 's child %d passed to 1#process",current->p_pid,procMgr.process[i].p_pid);
			procMgr.process[i].p_ppid = 1;
			if ( procMgr.process[i].p_stat == Process::SSTOP )
			{
				procMgr.process[i].SetRun();
			}
		}
	}

	procMgr.Swtch();
}

void Process::Clone(Process& proc)
{
	User& u = Kernel::Instance().GetUser();

	/* ����������Process�ṹ�еĴ󲿷����� */
	proc.p_size = this->p_size;
	proc.p_stat = Process::SRUN;
	proc.p_flag = Process::SLOAD;
	proc.p_uid = this->p_uid;
	proc.p_ttyp = this->p_ttyp;
	proc.p_nice = this->p_nice;
	proc.p_textp = this->p_textp;

	/* �������ӹ�ϵ */
	proc.p_pid = ProcessManager::NextUniquePid();
	proc.p_ppid = this->p_pid;

	/* ��ʼ�����̵�����س�Ա */
	proc.p_pri = 0;		/* ȷ��child����������С��������������ȸ��л���ռ��CPU */
	proc.p_time = 0;


	/* ���ļ����ƿ�File�ṹ���ü���+1 */
	for ( int i = 0; i < OpenFiles::NOFILES; i++ )
	{
		File* pFile;
		if ( (pFile = OpenFiles_get_file(i)) != NULL )
		{
			pFile->f_count++;
		}
	}
	/*
	 * GetF()����u.u_ofiles�еĿ��������������룬
	 * �粻��������½��̴���(fork)ϵͳ����ʧ�ܡ�
	 */
	User_get_error() = User::NOERROR;

	/* ���ӶԹ������Ķε����ü��� */
	if ( proc.p_textp != 0 )
	{
		proc.p_textp->x_count++;
		proc.p_textp->x_ccount++;
	}

	/* ���ӶԵ�ǰ����Ŀ¼�����ü��� */
	User_get_cdir()->i_count++;
}

//���ڶ�ջ���ʱ���Զ���չ��ջ
void Process::SStack()
{
	User& u = Kernel::Instance().GetUser();
	MemoryDescriptor& md = User_get_MemoryDescriptor();
	unsigned int change = 4096;
	//unsigned int change = 0;
	md.m_StackSize += change;
	unsigned int newSize = ProcessManager::USIZE + md.m_DataSize + md.m_StackSize;

	if ( false == User_get_MemoryDescriptor().EstablishUserPageTable(md.m_TextStartAddress,
						md.m_TextSize, md.m_DataStartAddress, md.m_DataSize, md.m_StackSize) )
	{
		User_get_error() = User::ENOMEM;
		return;
	}

	this->Expand(newSize);
	int dst = User_get_procp()->p_addr + newSize;
	unsigned int count = md.m_StackSize - change;
	while(count--)
	{
		dst--;
		Utility::CopySeg(dst - change, dst);
	}

	User_get_MemoryDescriptor().MapToPageTable();
}


void Process::SBreak()
{
	User& u = Kernel::Instance().GetUser();
	unsigned int newEnd = User_get_arg()[0];
	MemoryDescriptor& md = User_get_MemoryDescriptor();
	unsigned int newSize = newEnd - md.m_DataStartAddress;

	if (newEnd == 0)
	{
		User_get_ar0()[User::EAX] = md.m_DataStartAddress + md.m_DataSize;
		return;
	}

	if ( false == User_get_MemoryDescriptor().EstablishUserPageTable(md.m_TextStartAddress,
						md.m_TextSize, md.m_DataStartAddress, newSize, md.m_StackSize) )
	{
		//ϵͳ���ó���ʱ�������������ַ�ʽ���ء�ִ������·���ᵼ�� u.u_intflg == 1��User_get_error()�������޸�ΪEINTR��4�������ۺιʵ���ϵͳ����ʧ�ܡ�
		//aRetU(User_get_qsav());
		return;
	}

	int change = newSize - md.m_DataSize;
	md.m_DataSize = newSize;
	newSize += ProcessManager::USIZE + md.m_StackSize;

	/* ���ݶ���С */
	if ( change < 0 )
	{
		int dst = User_get_procp()->p_addr + newSize - md.m_StackSize;
		int count = md.m_StackSize;
		while(count--)
		{
			Utility::CopySeg(dst - change, dst);
			dst++;
		}
		this->Expand(newSize);
	}
	/* ���ݶ����� */
	else if ( change > 0 )
	{
		this->Expand(newSize);
		int dst = User_get_procp()->p_addr + newSize;
		int count = md.m_StackSize;
		while(count--)
		{
			dst--;
			Utility::CopySeg(dst - change, dst);
		}
	}
	User_get_ar0()[User::EAX] = md.m_DataStartAddress + md.m_DataSize;
}

extern "C" void compat_set_run(Process* proc) {
	proc->SetRun();
}

void Process::PSignal( int signal )
{
	Process_raise(this, signal);
}

int Process::IsSig()
{
	return Process_should_process(this);
}

/*
extern "C" void runtime();
extern "C" void SignalHandler();
*/

extern "C" void user_exit() {
	User_get_procp()->Exit();
}

void Process::PSig(struct pt_context* pContext) {
	Process_process_signal(this, pContext);
}

void Process::Nice() {
	Process_set_nice(this);
}

void Process::Ssig() {
	Process_send_signal(this);
}
